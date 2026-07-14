use std::collections::VecDeque;

use faer::mat;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use rand::seq::SliceRandom;
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

    let num_sides = rng.random_range(4..=8);
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
fn build_maze_arena(num_obstacles: usize, rng: &mut impl Rng) -> Arena {
    let cols = rng.random_range(5..=8);
    let rows = rng.random_range(3..=6);

    let x_bounds = random_partitions(cols, RMF[0], 2.0, rng);
    let y_bounds = random_partitions(rows, RMF[1], 1.5, rng);

    let total = cols * rows;
    let mut obstacles = vec![false; total];
    let mut indices: Vec<usize> = (0..total).collect();
    indices.shuffle(rng);

    let mut placed = 0;
    for &idx in &indices {
        if placed >= num_obstacles {
            break;
        }
        obstacles[idx] = true;
        if is_grid_connected(&obstacles, cols, rows) {
            placed += 1;
        } else {
            obstacles[idx] = false;
        }
    }

    let mut arena = Arena::new();
    for (idx, &is_obstacle) in obstacles.iter().enumerate() {
        if is_obstacle {
            let col = idx % cols;
            let row = idx / cols;
            let cell_x = x_bounds[col];
            let cell_y = y_bounds[row];
            let cell_w = x_bounds[col + 1] - x_bounds[col];
            let cell_h = y_bounds[row + 1] - y_bounds[row];
            arena.push(random_polygon_in_cell(cell_x, cell_y, cell_w, cell_h, rng));
        }
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

    let polygons = build_maze_arena(12, &mut rng);

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
}
