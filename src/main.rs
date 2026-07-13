use faer::mat;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use rand::Rng;

const RMF: [f64; 2] = [28.0, 15.0];

type Point = (f64, f64);
type PolygonPoints = Vec<Point>;
type Arena = Vec<PolygonPoints>;
type BoundingCircle = (f64, f64, f64);

fn draw_arena<DB>(
    chart: &mut ChartContext<'_, DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    arena: Arena,
) where
    DB: DrawingBackend,
{
    for polygon in arena {
        chart
            .draw_series(std::iter::once(Polygon::new(
                polygon,
                BLUE.mix(0.25).filled().stroke_width(2),
            )))
            .unwrap();
    }
}

fn generic_polygon_maker(bounding_circle: BoundingCircle, num_points: usize) -> PolygonPoints {
    let mut rng = rand::rng();
    let mut angles: Vec<f64> = (0..num_points)
        .map(|_| rng.random_range(0.0..std::f64::consts::TAU))
        .collect();

    angles.sort_by(|left, right| left.total_cmp(right));

    let mut polygon = PolygonPoints::with_capacity(num_points);

    for angle in angles {
        // sqrt makes points uniform by area within the bounding circle.
        let distance = bounding_circle.2 * rng.random::<f64>().sqrt();
        let x = bounding_circle.0 + distance * angle.cos();
        let y = bounding_circle.1 + distance * angle.sin();
        polygon.push((x, y));
    }

    polygon
}

// Placeholder for the future smooth-boundary implementation.
#[allow(dead_code)]
fn spline_maker(bounding_box: PolygonPoints) -> PolygonPoints {
    bounding_box
}

fn make_random_bounding_circles(number_of_circles: usize) -> Vec<BoundingCircle> {
    let mut rng = rand::rng();
    let center_points: Vec<Point> = (0..number_of_circles)
        .map(|_| (rng.random_range(0.0..RMF[0]), rng.random_range(0.0..RMF[1])))
        .collect();

    let mut bounding_circles = Vec::with_capacity(number_of_circles);

    for &point in &center_points {
        let wall_distance = point
            .0
            .min(RMF[0] - point.0)
            .min(point.1)
            .min(RMF[1] - point.1);

        let radius = rng.random_range(0.0..RMF[0] / 2.0).min(wall_distance);

        bounding_circles.push((point.0, point.1, radius));
    }

    bounding_circles
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


    let number_of_random_polygons = 10;
    let polygons: Arena = make_random_bounding_circles(number_of_random_polygons)
        .into_iter()
        .map(|circle| generic_polygon_maker(circle, rng.random_range(3..20)))
        .collect();

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

    draw_arena(&mut chart, polygons);
    map.present().unwrap();
}
