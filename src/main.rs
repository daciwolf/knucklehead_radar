use std::collections::VecDeque;

use faer::mat;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use rand::Rng;

const RMF: [f64; 2] = [28.0, 15.0];
const CELL_PADDING: f64 = 0.3;

type Point = (f64, f64);
type PolygonPoints = Vec<Point>;
type Arena = Vec<PolygonPoints>;

fn draw_arena<DB>(
    chart: &mut ChartContext<'_, DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    arena: &Arena,
) where
    DB: DrawingBackend,
{
    for polygon in arena {
        chart
            .draw_series(std::iter::once(Polygon::new(
                polygon.clone(),
                BLUE.mix(0.25).filled().stroke_width(2),
            )))
            .unwrap();
    }
}

/// BFS flood-fill to verify every open cell is still reachable from every other.
fn is_grid_connected(obstacles: &[bool], cols: usize, rows: usize) -> bool {
    let total = cols * rows;
    let start = match (0..total).find(|&i| !obstacles[i]) {
        Some(s) => s,
        None => return true,
    };

    let mut visited = vec![false; total];
    let mut queue = VecDeque::new();
    queue.push_back(start);
    visited[start] = true;
    let mut count = 1usize;

    while let Some(idx) = queue.pop_front() {
        let col = idx % cols;
        let row = idx / cols;
        for (dc, dr) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            if nc >= 0 && nc < cols as i32 && nr >= 0 && nr < rows as i32 {
                let ni = nr as usize * cols + nc as usize;
                if !obstacles[ni] && !visited[ni] {
                    visited[ni] = true;
                    count += 1;
                    queue.push_back(ni);
                }
            }
        }
    }

    count == obstacles.iter().filter(|&&o| !o).count()
}

/// Random convex polygon confined to a single grid cell (with padding).
fn random_polygon_in_cell(
    cell_x: f64,
    cell_y: f64,
    cell_w: f64,
    cell_h: f64,
    rng: &mut impl Rng,
) -> PolygonPoints {
    let px = cell_x + CELL_PADDING;
    let py = cell_y + CELL_PADDING;
    let pw = cell_w - 2.0 * CELL_PADDING;
    let ph = cell_h - 2.0 * CELL_PADDING;

    let cx = px + pw / 2.0;
    let cy = py + ph / 2.0;

    let num_sides = rng.random_range(3..=20);
    let scale = rng.random_range(0.6..=1.0_f64);

    let mut angles: Vec<f64> = (0..num_sides)
        .map(|_| rng.random_range(0.0..std::f64::consts::TAU))
        .collect();
    angles.sort_by(|a, b| a.total_cmp(b));

    angles
        .iter()
        .map(|&angle| {
            let rx = pw / 2.0 * scale;
            let ry = ph / 2.0 * scale;
            (cx + rx * angle.cos(), cy + ry * angle.sin())
        })
        .collect()
}

/// Place obstacles on an irregular grid, rejecting any placement that would
/// disconnect open space.
fn build_arena_with_obstacles(num_obstacles: usize, rng: &mut impl Rng, potential_sizes_of_polygons: (f64, f64)) -> Arena {
    let mut arena = Arena::new();
    for _ in 0..num_obstacles {
        let cell_point: Point = (rng.random_range(0.0..RMF[0]), rng.random_range(0.0..RMF[1]));
        let cell_width: f64 = rng.random_range(potential_sizes_of_polygons.0..=potential_sizes_of_polygons.1);
        let cell_height: f64 = rng.random_range(potential_sizes_of_polygons.0..=potential_sizes_of_polygons.1);
        let cell_polygon: PolygonPoints = random_polygon_in_cell(cell_point.0, cell_point.1, cell_width, cell_height, rng);
        arena.push(cell_polygon);
    }
    return arena;
}

fn random_circle(cx: f64, cy: f64, radius: f64) -> PolygonPoints {
    let segments = 32;
    (0..segments)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / segments as f64;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect()
}

fn random_square(cx: f64, cy: f64, half_size: f64) -> PolygonPoints {
    vec![
        (cx - half_size, cy - half_size),
        (cx + half_size, cy - half_size),
        (cx + half_size, cy + half_size),
        (cx - half_size, cy + half_size),
    ]
}

fn random_rectangle(cx: f64, cy: f64, half_w: f64, half_h: f64) -> PolygonPoints {
    vec![
        (cx - half_w, cy - half_h),
        (cx + half_w, cy - half_h),
        (cx + half_w, cy + half_h),
        (cx - half_w, cy + half_h),
    ]
}

type Aabb = (f64, f64, f64, f64);

fn polygon_aabb(poly: &PolygonPoints) -> Aabb {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &(x, y) in poly {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (min_x, min_y, max_x, max_y)
}

fn aabbs_overlap(a: &Aabb, b: &Aabb, gap: f64) -> bool {
    a.0 < b.2 + gap && a.2 + gap > b.0 && a.1 < b.3 + gap && a.3 + gap > b.1
}

fn build_arena_with_simple_shapes(
    num_obstacles: usize,
    rng: &mut impl Rng,
    potential_sizes: (f64, f64),
    bounds: (f64, f64),
) -> Arena {
    let mut arena = Arena::new();
    let mut placed_aabbs: Vec<Aabb> = Vec::new();
    let gap = 0.3;
    let max_attempts = 200;

    let mut placed = 0;
    let mut attempts = 0;
    while placed < num_obstacles && attempts < max_attempts {
        attempts += 1;
        let cx: f64 = rng.random_range(0.0..bounds.0);
        let cy: f64 = rng.random_range(0.0..bounds.1);
        let shape: u32 = rng.random_range(0..3);
        let polygon = match shape {
            0 => {
                let radius = rng.random_range(potential_sizes.0..=potential_sizes.1) / 2.0;
                random_circle(cx, cy, radius)
            }
            1 => {
                let half_size = rng.random_range(potential_sizes.0..=potential_sizes.1) / 2.0;
                random_square(cx, cy, half_size)
            }
            _ => {
                let half_w = rng.random_range(potential_sizes.0..=potential_sizes.1) / 2.0;
                let half_h = rng.random_range(potential_sizes.0..=potential_sizes.1) / 2.0;
                random_rectangle(cx, cy, half_w, half_h)
            }
        };

        let aabb = polygon_aabb(&polygon);
        if aabb.0 < 0.0 || aabb.1 < 0.0 || aabb.2 > bounds.0 || aabb.3 > bounds.1 {
            continue;
        }
        if placed_aabbs.iter().any(|existing| aabbs_overlap(existing, &aabb, gap)) {
            continue;
        }

        placed_aabbs.push(aabb);
        arena.push(polygon);
        placed += 1;
    }
    arena
}



fn main() {
    // Each column is one field point: (x0, y0), (x1, y1), ...
    let rmf_points = mat![[0.0, 0.0, RMF[0], RMF[0]], [0.0, RMF[1], RMF[1], 0.0],];

    let field: PolygonPoints = (0..rmf_points.ncols())
        .map(|column| {
            let x = rmf_points[(0, column)];
            let y = rmf_points[(1, column)];
            (x, y)
        })
        .collect();
    
    let mut rng = rand::rng();

    let polygons = build_arena_with_obstacles(12, &mut rng, (1.0, 4.0));

    let map = BitMapBackend::new("RMF_ARENA.png", (840, 450)).into_drawing_area();
    map.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&map)
        .caption("Robomaster Field", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0.0..RMF[0], 0.0..RMF[1])
        .unwrap();

    chart
        .configure_mesh()
        .x_desc("x")
        .y_desc("y")
        .x_labels(15)
        .y_labels(9)
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            field.iter().copied().chain(std::iter::once(field[0])),
            BLACK.stroke_width(3),
        ))
        .unwrap();

    draw_arena(&mut chart, &polygons);
    map.present().unwrap();

    let simple_polygons = build_arena_with_simple_shapes(12, &mut rng, (1.0, 4.0), (RMF[0], RMF[1]));

    let map2 = BitMapBackend::new("RMF_ARENA_SIMPLE.png", (840, 450)).into_drawing_area();
    map2.fill(&WHITE).unwrap();

    let mut chart2 = ChartBuilder::on(&map2)
        .caption("Robomaster Field (Simple Shapes)", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0.0..RMF[0], 0.0..RMF[1])
        .unwrap();

    chart2
        .configure_mesh()
        .x_desc("x")
        .y_desc("y")
        .x_labels(15)
        .y_labels(9)
        .draw()
        .unwrap();

    chart2
        .draw_series(LineSeries::new(
            field.iter().copied().chain(std::iter::once(field[0])),
            BLACK.stroke_width(3),
        ))
        .unwrap();

    draw_arena(&mut chart2, &simple_polygons);
    map2.present().unwrap();
}
