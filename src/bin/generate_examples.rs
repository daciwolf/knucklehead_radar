use std::collections::VecDeque;
use std::fs;

use noise::{NoiseFn, Perlin};
use plotters::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const BOUNDS: (f64, f64) = (28.0, 15.0);

// ═══════════════════════════════════════════════════════════════
//  Rendering
// ═══════════════════════════════════════════════════════════════

fn render_grid(grid: &[Vec<bool>], filename: &str, title: &str) {
    let rows = grid.len();
    if rows == 0 {
        return;
    }
    let cols = grid[0].len();
    let cell_w = BOUNDS.0 / cols as f64;
    let cell_h = BOUNDS.1 / rows as f64;

    let map = BitMapBackend::new(filename, (840, 450)).into_drawing_area();
    map.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&map)
        .caption(title, ("sans-serif", 14))
        .margin(10)
        .x_label_area_size(25)
        .y_label_area_size(25)
        .build_cartesian_2d(0.0..BOUNDS.0, 0.0..BOUNDS.1)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    let mut walls = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            if grid[r][c] {
                let x0 = c as f64 * cell_w;
                let y0 = (rows - 1 - r) as f64 * cell_h;
                walls.push(Rectangle::new(
                    [(x0, y0), (x0 + cell_w, y0 + cell_h)],
                    BLUE.mix(0.6).filled(),
                ));
            }
        }
    }
    chart.draw_series(walls).unwrap();

    chart
        .draw_series(LineSeries::new(
            vec![
                (0.0, 0.0),
                (BOUNDS.0, 0.0),
                (BOUNDS.0, BOUNDS.1),
                (0.0, BOUNDS.1),
                (0.0, 0.0),
            ],
            BLACK.stroke_width(2),
        ))
        .unwrap();

    map.present().unwrap();
}

// ═══════════════════════════════════════════════════════════════
//  1. Cellular Automata
// ═══════════════════════════════════════════════════════════════

fn cellular_automata(
    cols: usize,
    rows: usize,
    fill_ratio: f64,
    smooth_iters: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<bool>> {
    let mut grid: Vec<Vec<bool>> = (0..rows)
        .map(|_| (0..cols).map(|_| rng.random::<f64>() < fill_ratio).collect())
        .collect();

    for _ in 0..smooth_iters {
        let prev = grid.clone();
        for r in 0..rows {
            for c in 0..cols {
                let mut wall_count = 0i32;
                for dr in -1..=1i32 {
                    for dc in -1..=1i32 {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let nr = r as i32 + dr;
                        let nc = c as i32 + dc;
                        if nr < 0
                            || nr >= rows as i32
                            || nc < 0
                            || nc >= cols as i32
                            || prev[nr as usize][nc as usize]
                        {
                            wall_count += 1;
                        }
                    }
                }
                grid[r][c] = wall_count >= 5;
            }
        }
    }

    grid
}

// ═══════════════════════════════════════════════════════════════
//  2. Noise Threshold (Perlin)
// ═══════════════════════════════════════════════════════════════

fn noise_threshold(
    cols: usize,
    rows: usize,
    frequency: f64,
    threshold: f64,
    seed: u32,
) -> Vec<Vec<bool>> {
    let perlin = Perlin::new(seed);
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    let x = c as f64 / cols as f64 * frequency;
                    let y = r as f64 / rows as f64 * frequency;
                    perlin.get([x, y]) > threshold
                })
                .collect()
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════
//  3. Binary Space Partitioning
// ═══════════════════════════════════════════════════════════════

fn bsp_generate(
    cols: usize,
    rows: usize,
    max_depth: usize,
    min_room_ratio: f64,
    corridor_width: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<bool>> {
    let mut grid = vec![vec![true; cols]; rows];
    bsp_split(
        &mut grid, 0, 0, cols, rows, 0, max_depth, min_room_ratio, corridor_width, rng,
    );
    grid
}

fn bsp_split(
    grid: &mut [Vec<bool>],
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    depth: usize,
    max_depth: usize,
    min_room_ratio: f64,
    corridor_width: usize,
    rng: &mut impl Rng,
) -> (usize, usize) {
    let grid_h = grid.len();
    let grid_w = grid[0].len();

    if depth >= max_depth || w < 8 || h < 8 {
        let rw = ((w as f64 * rng.random_range(min_room_ratio..0.85)) as usize)
            .clamp(2, w.saturating_sub(2).max(2));
        let rh = ((h as f64 * rng.random_range(min_room_ratio..0.85)) as usize)
            .clamp(2, h.saturating_sub(2).max(2));
        let max_ox = w.saturating_sub(rw + 1).max(1);
        let max_oy = h.saturating_sub(rh + 1).max(1);
        let rx = x + rng.random_range(1..=max_ox);
        let ry = y + rng.random_range(1..=max_oy);
        for row in ry..(ry + rh).min(grid_h) {
            for col in rx..(rx + rw).min(grid_w) {
                grid[row][col] = false;
            }
        }
        return (
            (rx + rw / 2).min(grid_w - 1),
            (ry + rh / 2).min(grid_h - 1),
        );
    }

    if w >= h {
        let lo = (w / 3).max(1);
        let hi = (2 * w / 3).max(lo);
        let split_at = rng.random_range(lo..=hi);
        let (cx1, cy1) =
            bsp_split(grid, x, y, split_at, h, depth + 1, max_depth, min_room_ratio, corridor_width, rng);
        let right_x = (x + split_at).min(grid_w.saturating_sub(1));
        let right_w = w.saturating_sub(split_at).max(4);
        let (cx2, cy2) =
            bsp_split(grid, right_x, y, right_w, h, depth + 1, max_depth, min_room_ratio, corridor_width, rng);
        carve_corridor(grid, cx1, cy1, cx2, cy2, corridor_width);
        ((cx1 + cx2) / 2, (cy1 + cy2) / 2)
    } else {
        let lo = (h / 3).max(1);
        let hi = (2 * h / 3).max(lo);
        let split_at = rng.random_range(lo..=hi);
        let (cx1, cy1) =
            bsp_split(grid, x, y, w, split_at, depth + 1, max_depth, min_room_ratio, corridor_width, rng);
        let bot_y = (y + split_at).min(grid_h.saturating_sub(1));
        let bot_h = h.saturating_sub(split_at).max(4);
        let (cx2, cy2) =
            bsp_split(grid, x, bot_y, w, bot_h, depth + 1, max_depth, min_room_ratio, corridor_width, rng);
        carve_corridor(grid, cx1, cy1, cx2, cy2, corridor_width);
        ((cx1 + cx2) / 2, (cy1 + cy2) / 2)
    }
}

fn carve_corridor(
    grid: &mut [Vec<bool>],
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    width: usize,
) {
    let grid_h = grid.len();
    let grid_w = grid[0].len();

    let (sx, ex) = (x1.min(x2), x1.max(x2));
    for col in sx..=ex.min(grid_w - 1) {
        for dw in 0..width {
            let row = y1 + dw;
            if row < grid_h {
                grid[row][col] = false;
            }
        }
    }
    let (sy, ey) = (y1.min(y2), y1.max(y2));
    for row in sy..=ey.min(grid_h - 1) {
        for dw in 0..width {
            let col = x2 + dw;
            if col < grid_w {
                grid[row][col] = false;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  4. Wave Function Collapse
// ═══════════════════════════════════════════════════════════════

struct WfcTile {
    edges: [u8; 4], // N, E, S, W — 0 = open, 1 = wall
    weight: f64,
    pattern: [[bool; 3]; 3],
}

fn wfc_tiles(empty_w: f64, wall_w: f64) -> Vec<WfcTile> {
    let f = false;
    let t = true;
    vec![
        WfcTile { edges: [0, 0, 0, 0], weight: empty_w,       pattern: [[f,f,f],[f,f,f],[f,f,f]] },
        WfcTile { edges: [1, 1, 1, 1], weight: wall_w * 0.3,  pattern: [[t,t,t],[t,t,t],[t,t,t]] },
        WfcTile { edges: [0, 1, 0, 1], weight: wall_w,        pattern: [[f,f,f],[t,t,t],[f,f,f]] },
        WfcTile { edges: [1, 0, 1, 0], weight: wall_w,        pattern: [[f,t,f],[f,t,f],[f,t,f]] },
        WfcTile { edges: [1, 1, 0, 0], weight: wall_w,        pattern: [[f,t,f],[f,t,t],[f,f,f]] },
        WfcTile { edges: [0, 1, 1, 0], weight: wall_w,        pattern: [[f,f,f],[f,t,t],[f,t,f]] },
        WfcTile { edges: [0, 0, 1, 1], weight: wall_w,        pattern: [[f,f,f],[t,t,f],[f,t,f]] },
        WfcTile { edges: [1, 0, 0, 1], weight: wall_w,        pattern: [[f,t,f],[t,t,f],[f,f,f]] },
        WfcTile { edges: [1, 1, 0, 1], weight: wall_w * 0.5,  pattern: [[f,t,f],[t,t,t],[f,f,f]] },
        WfcTile { edges: [1, 1, 1, 0], weight: wall_w * 0.5,  pattern: [[f,t,f],[f,t,t],[f,t,f]] },
        WfcTile { edges: [0, 1, 1, 1], weight: wall_w * 0.5,  pattern: [[f,f,f],[t,t,t],[f,t,f]] },
        WfcTile { edges: [1, 0, 1, 1], weight: wall_w * 0.5,  pattern: [[f,t,f],[t,t,f],[f,t,f]] },
    ]
}

fn wfc_generate(
    cols: usize,
    rows: usize,
    tiles: &[WfcTile],
    rng: &mut impl Rng,
) -> Option<Vec<Vec<usize>>> {
    let n = tiles.len();
    let mut possible = vec![vec![vec![true; n]; cols]; rows];
    let mut collapsed: Vec<Vec<Option<usize>>> = vec![vec![None; cols]; rows];

    for _ in 0..(cols * rows) {
        let mut best = usize::MAX;
        let mut candidates = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                if collapsed[r][c].is_some() {
                    continue;
                }
                let count = possible[r][c].iter().filter(|&&p| p).count();
                if count == 0 {
                    return None;
                }
                if count < best {
                    best = count;
                    candidates = vec![(r, c)];
                } else if count == best {
                    candidates.push((r, c));
                }
            }
        }
        if candidates.is_empty() {
            break;
        }

        let &(r, c) = &candidates[rng.random_range(0..candidates.len())];

        let options: Vec<(usize, f64)> = possible[r][c]
            .iter()
            .enumerate()
            .filter(|&(_, p)| *p)
            .map(|(i, _)| (i, tiles[i].weight))
            .collect();
        let total_w: f64 = options.iter().map(|o| o.1).sum();
        let mut roll = rng.random::<f64>() * total_w;
        let mut chosen = options[0].0;
        for &(idx, w) in &options {
            roll -= w;
            if roll <= 0.0 {
                chosen = idx;
                break;
            }
        }

        collapsed[r][c] = Some(chosen);
        for i in 0..n {
            possible[r][c][i] = i == chosen;
        }

        // Constraint propagation via BFS
        let mut queue = VecDeque::new();
        queue.push_back((r, c));
        while let Some((pr, pc)) = queue.pop_front() {
            // (neighbor_row, neighbor_col, my_edge_index, their_edge_index)
            let neighbors = [
                (pr.wrapping_sub(1), pc, 0usize, 2usize),
                (pr, pc + 1, 1, 3),
                (pr + 1, pc, 2, 0),
                (pr, pc.wrapping_sub(1), 3, 1),
            ];
            for &(nr, nc, my_edge, their_edge) in &neighbors {
                if nr >= rows || nc >= cols || collapsed[nr][nc].is_some() {
                    continue;
                }
                let mut allowed = [false; 2]; // edge values are 0 or 1
                for (i, &p) in possible[pr][pc].iter().enumerate() {
                    if p {
                        allowed[tiles[i].edges[my_edge] as usize] = true;
                    }
                }
                let mut changed = false;
                for i in 0..n {
                    if possible[nr][nc][i] && !allowed[tiles[i].edges[their_edge] as usize] {
                        possible[nr][nc][i] = false;
                        changed = true;
                    }
                }
                if changed {
                    queue.push_back((nr, nc));
                }
            }
        }
    }

    Some(
        collapsed
            .into_iter()
            .map(|row| row.into_iter().map(|c| c.unwrap_or(0)).collect())
            .collect(),
    )
}

fn wfc_to_grid(result: &[Vec<usize>], tiles: &[WfcTile]) -> Vec<Vec<bool>> {
    let wfc_rows = result.len();
    let wfc_cols = result[0].len();
    let mut grid = vec![vec![false; wfc_cols * 3]; wfc_rows * 3];
    for r in 0..wfc_rows {
        for c in 0..wfc_cols {
            let pat = &tiles[result[r][c]].pattern;
            for pr in 0..3 {
                for pc in 0..3 {
                    grid[r * 3 + pr][c * 3 + pc] = pat[pr][pc];
                }
            }
        }
    }
    grid
}

// ═══════════════════════════════════════════════════════════════
//  4b. WFC Amended — keep the random pockets, drop the segmenting walls
// ═══════════════════════════════════════════════════════════════

/// Label every connected open region (4-connectivity). Wall cells stay -1.
fn label_open_regions(wall_grid: &[Vec<bool>]) -> (Vec<Vec<i32>>, usize) {
    let rows = wall_grid.len();
    let cols = wall_grid[0].len();
    let mut labels = vec![vec![-1i32; cols]; rows];
    let mut next: i32 = 0;

    for sr in 0..rows {
        for sc in 0..cols {
            if wall_grid[sr][sc] || labels[sr][sc] != -1 {
                continue;
            }
            let mut queue = VecDeque::new();
            queue.push_back((sr, sc));
            labels[sr][sc] = next;
            while let Some((r, c)) = queue.pop_front() {
                for (dr, dc) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                        continue;
                    }
                    let (nr, nc) = (nr as usize, nc as usize);
                    if !wall_grid[nr][nc] && labels[nr][nc] == -1 {
                        labels[nr][nc] = next;
                        queue.push_back((nr, nc));
                    }
                }
            }
            next += 1;
        }
    }

    (labels, next as usize)
}

/// Two open regions count as "right next to" each other if they're separated by
/// a straight orthogonal run of at most `max_wall` wall cells. Returns a
/// symmetric region-by-region adjacency matrix.
fn region_adjacency(labels: &[Vec<i32>], num_regions: usize, max_wall: usize) -> Vec<Vec<bool>> {
    let rows = labels.len();
    let cols = labels[0].len();
    let mut adj = vec![vec![false; num_regions]; num_regions];

    for r in 0..rows {
        for c in 0..cols {
            let a = labels[r][c];
            if a < 0 {
                continue;
            }
            // Only look right and down; adjacency is marked symmetrically.
            for &(dr, dc) in &[(0i32, 1i32), (1, 0)] {
                let mut walls = 0usize;
                let mut k = 1i32;
                loop {
                    let nr = r as i32 + dr * k;
                    let nc = c as i32 + dc * k;
                    if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                        break;
                    }
                    let b = labels[nr as usize][nc as usize];
                    if b < 0 {
                        walls += 1;
                        if walls > max_wall {
                            break;
                        }
                        k += 1;
                        continue;
                    }
                    if walls >= 1 && b != a {
                        adj[a as usize][b as usize] = true;
                        adj[b as usize][a as usize] = true;
                    }
                    break;
                }
            }
        }
    }

    adj
}

/// Take a WFC wall grid, throw `num_points` random points, and fill in the
/// enclosed pocket each point is trapped inside. Everything that wasn't filled
/// (including the original WFC walls) is cleared, so the leftover open space is
/// fully connected. `border_thickness` grows a solid border of the given size
/// around each filled pocket (0 = remove border / no border at all).
fn wfc_amended(
    wall_grid: &[Vec<bool>],
    num_points: usize,
    border_thickness: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<bool>> {
    let rows = wall_grid.len();
    let cols = wall_grid[0].len();
    let (labels, num_regions) = label_open_regions(wall_grid);

    // A region that touches the outer edge isn't really "trapped".
    let mut touches_border = vec![false; num_regions.max(1)];
    for r in 0..rows {
        for c in 0..cols {
            let id = labels[r][c];
            if id >= 0 && (r == 0 || c == 0 || r == rows - 1 || c == cols - 1) {
                touches_border[id as usize] = true;
            }
        }
    }

    // With a border, adjacent filled pockets grow into each other and merge, so
    // we forbid picking a pocket right next to an already-picked one. The spacing
    // scales with the border size (two blobs each grown by N need a >2N gap).
    // When the "remove border" option is on (thickness == 0) the restriction is
    // lifted, since there's nothing to grow into a neighbor.
    let adjacency = if border_thickness > 0 {
        Some(region_adjacency(&labels, num_regions.max(1), 2 * border_thickness))
    } else {
        None
    };

    // Throw random points; keep the enclosed pocket each one lands in.
    let mut selected = vec![false; num_regions.max(1)];
    let mut selected_ids: Vec<usize> = Vec::new();
    let mut chosen = 0;
    let mut attempts = 0;
    let max_attempts = num_points * 200 + 500;
    while chosen < num_points && attempts < max_attempts {
        attempts += 1;
        let r = rng.random_range(0..rows);
        let c = rng.random_range(0..cols);
        let id = labels[r][c];
        if id < 0 {
            continue; // landed on a wall
        }
        let id = id as usize;
        if touches_border[id] || selected[id] {
            continue; // not trapped, or already picked
        }
        if let Some(adj) = &adjacency {
            if selected_ids.iter().any(|&s| adj[id][s]) {
                continue; // right next to an already-picked pocket
            }
        }
        selected[id] = true;
        selected_ids.push(id);
        chosen += 1;
    }

    // Fill the selected pockets; drop everything else.
    let mut result = vec![vec![false; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let id = labels[r][c];
            if id >= 0 && selected[id as usize] {
                result[r][c] = true;
            }
        }
    }

    // Grow a solid border of `border_thickness` cells around each filled pocket
    // by dilating the filled set (Chebyshev distance). Thickness 0 leaves the
    // bare pockets with no border.
    if border_thickness > 0 {
        let mut filled: Vec<(usize, usize)> = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                if result[r][c] {
                    filled.push((r, c));
                }
            }
        }
        let n = border_thickness as i32;
        for (fr, fc) in filled {
            for dr in -n..=n {
                for dc in -n..=n {
                    let nr = fr as i32 + dr;
                    let nc = fc as i32 + dc;
                    if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                        result[nr as usize][nc as usize] = true;
                    }
                }
            }
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════
//  Main — generate images (5 approaches × 10 params × 10 variations)
// ═══════════════════════════════════════════════════════════════

fn main() {
    fs::create_dir_all("output/cellular_automata").unwrap();
    fs::create_dir_all("output/noise_threshold").unwrap();
    fs::create_dir_all("output/bsp").unwrap();
    fs::create_dir_all("output/wfc").unwrap();
    fs::create_dir_all("output/wfc_ammended").unwrap();
    fs::create_dir_all("output/wfc_ammended_borders").unwrap();

    // Selective regeneration:
    //   "amended" / --amended-only  → only output/wfc_ammended (100)
    //   "borders" / --borders-only  → only output/wfc_ammended_borders (1000)
    // no arg → generate everything.
    let args: Vec<String> = std::env::args().collect();
    let amended_only = args.iter().any(|a| a == "amended" || a == "--amended-only");
    let borders_only = args.iter().any(|a| a == "borders" || a == "--borders-only");
    let run_all = !amended_only && !borders_only;

    if run_all {
    // ── 1. Cellular Automata ──────────────────────────────────
    // (fill_ratio, smooth_iterations, grid_cols, grid_rows)
    let ca_params: [(f64, usize, usize, usize); 10] = [
        (0.30, 3, 56, 30),
        (0.35, 3, 56, 30),
        (0.40, 4, 56, 30),
        (0.45, 4, 56, 30),
        (0.50, 5, 56, 30),
        (0.35, 5, 84, 45),
        (0.40, 3, 84, 45),
        (0.45, 5, 84, 45),
        (0.40, 4, 140, 75),
        (0.48, 6, 140, 75),
    ];
    println!("Generating Cellular Automata (100 images)...");
    for (pi, &(fill, smooth, cols, rows)) in ca_params.iter().enumerate() {
        for vi in 0..10 {
            let seed = (pi * 100 + vi) as u64;
            let mut rng = StdRng::seed_from_u64(seed);
            let grid = cellular_automata(cols, rows, fill, smooth, &mut rng);
            let file = format!("output/cellular_automata/p{:02}_v{:02}.png", pi + 1, vi + 1);
            let title = format!(
                "CA fill={:.0}% smooth={} {}x{} #{}",
                fill * 100.0,
                smooth,
                cols,
                rows,
                vi + 1
            );
            render_grid(&grid, &file, &title);
        }
        println!("  param set {}/10 done", pi + 1);
    }

    // ── 2. Noise Threshold ────────────────────────────────────
    // (frequency, threshold, grid_cols, grid_rows)
    let noise_params: [(f64, f64, usize, usize); 10] = [
        (4.0, 0.0, 56, 30),
        (4.0, 0.15, 56, 30),
        (6.0, 0.0, 56, 30),
        (6.0, -0.10, 56, 30),
        (8.0, 0.0, 84, 45),
        (8.0, 0.10, 84, 45),
        (10.0, 0.0, 84, 45),
        (12.0, -0.10, 84, 45),
        (10.0, 0.05, 140, 75),
        (15.0, 0.0, 140, 75),
    ];
    println!("Generating Noise Threshold (100 images)...");
    for (pi, &(freq, thresh, cols, rows)) in noise_params.iter().enumerate() {
        for vi in 0..10 {
            let seed = (pi * 100 + vi) as u32;
            let grid = noise_threshold(cols, rows, freq, thresh, seed);
            let file = format!("output/noise_threshold/p{:02}_v{:02}.png", pi + 1, vi + 1);
            let title = format!(
                "Noise f={:.0} t={:.2} {}x{} #{}",
                freq,
                thresh,
                cols,
                rows,
                vi + 1
            );
            render_grid(&grid, &file, &title);
        }
        println!("  param set {}/10 done", pi + 1);
    }

    // ── 3. BSP ────────────────────────────────────────────────
    // (max_depth, min_room_ratio, corridor_width, grid_cols, grid_rows)
    let bsp_params: [(usize, f64, usize, usize, usize); 10] = [
        (3, 0.40, 1, 56, 30),
        (3, 0.60, 2, 56, 30),
        (4, 0.40, 1, 56, 30),
        (4, 0.60, 2, 56, 30),
        (5, 0.40, 1, 84, 45),
        (5, 0.50, 2, 84, 45),
        (4, 0.30, 3, 84, 45),
        (6, 0.40, 1, 84, 45),
        (5, 0.40, 2, 140, 75),
        (6, 0.50, 2, 140, 75),
    ];
    println!("Generating BSP (100 images)...");
    for (pi, &(depth, ratio, corridor, cols, rows)) in bsp_params.iter().enumerate() {
        for vi in 0..10 {
            let seed = (pi * 100 + vi) as u64;
            let mut rng = StdRng::seed_from_u64(seed);
            let grid = bsp_generate(cols, rows, depth, ratio, corridor, &mut rng);
            let file = format!("output/bsp/p{:02}_v{:02}.png", pi + 1, vi + 1);
            let title = format!(
                "BSP d={} r={:.1} c={} {}x{} #{}",
                depth,
                ratio,
                corridor,
                cols,
                rows,
                vi + 1
            );
            render_grid(&grid, &file, &title);
        }
        println!("  param set {}/10 done", pi + 1);
    }

    // ── 4. WFC ────────────────────────────────────────────────
    // (wfc_cols, wfc_rows, empty_weight, wall_weight)
    // Final grid is 3x the WFC dimensions (each tile expands to 3×3)
    let wfc_params: [(usize, usize, f64, f64); 10] = [
        (10, 6, 6.0, 1.0),
        (10, 6, 4.0, 1.0),
        (10, 6, 2.0, 1.0),
        (14, 8, 6.0, 1.0),
        (14, 8, 3.0, 1.0),
        (14, 8, 1.5, 1.0),
        (20, 10, 5.0, 1.0),
        (20, 10, 2.5, 1.0),
        (20, 10, 1.0, 1.0),
        (28, 15, 3.0, 1.0),
    ];
    println!("Generating WFC (100 images)...");
    for (pi, &(wfc_c, wfc_r, ew, ww)) in wfc_params.iter().enumerate() {
        let tiles = wfc_tiles(ew, ww);
        for vi in 0..10 {
            let mut grid = None;
            for attempt in 0..50u64 {
                let seed = pi as u64 * 10_000 + vi as u64 * 100 + attempt;
                let mut rng = StdRng::seed_from_u64(seed);
                if let Some(result) = wfc_generate(wfc_c, wfc_r, &tiles, &mut rng) {
                    grid = Some(wfc_to_grid(&result, &tiles));
                    break;
                }
            }
            if let Some(ref g) = grid {
                let file = format!("output/wfc/p{:02}_v{:02}.png", pi + 1, vi + 1);
                let title = format!(
                    "WFC {}x{} ew={:.0} ww={:.0} #{}",
                    wfc_c,
                    wfc_r,
                    ew,
                    ww,
                    vi + 1
                );
                render_grid(g, &file, &title);
            } else {
                eprintln!("WFC failed: param set {} variation {}", pi + 1, vi + 1);
            }
        }
        println!("  param set {}/10 done", pi + 1);
    }
    } // end run_all

    // ── 4b. WFC Amended ───────────────────────────────────────
    // (wfc_cols, wfc_rows, empty_weight, wall_weight, num_points, border_thickness)
    // Heavy wall weights so WFC segments the field into lots of pockets,
    // then random points decide which trapped pockets become solid obstacles.
    if run_all || amended_only {
    let wfc_am_params: [(usize, usize, f64, f64, usize, usize); 10] = [
        (10, 6, 2.0, 1.0, 3, 0),
        (10, 6, 2.0, 1.0, 5, 1),
        (14, 8, 2.0, 1.0, 6, 0),
        (14, 8, 1.5, 1.0, 8, 1),
        (14, 8, 1.5, 1.0, 6, 0),
        (20, 10, 2.0, 1.0, 8, 0),
        (20, 10, 1.5, 1.0, 10, 1),
        (20, 10, 1.0, 1.0, 6, 0),
        (28, 15, 1.5, 1.0, 12, 1),
        (28, 15, 1.0, 1.0, 10, 0),
    ];
    println!("Generating WFC Amended (100 images)...");
    for (pi, &(wfc_c, wfc_r, ew, ww, num_points, border)) in wfc_am_params.iter().enumerate() {
        let tiles = wfc_tiles(ew, ww);
        for vi in 0..10 {
            let mut wall_grid = None;
            for attempt in 0..50u64 {
                let seed = 500_000 + pi as u64 * 10_000 + vi as u64 * 100 + attempt;
                let mut rng = StdRng::seed_from_u64(seed);
                if let Some(result) = wfc_generate(wfc_c, wfc_r, &tiles, &mut rng) {
                    wall_grid = Some(wfc_to_grid(&result, &tiles));
                    break;
                }
            }
            if let Some(wg) = wall_grid {
                let mut rng = StdRng::seed_from_u64(900_000 + pi as u64 * 1000 + vi as u64);
                let grid = wfc_amended(&wg, num_points, border, &mut rng);
                let file = format!("output/wfc_ammended/p{:02}_v{:02}.png", pi + 1, vi + 1);
                let title = format!(
                    "WFC-Am {}x{} pts={} border={} #{}",
                    wfc_c,
                    wfc_r,
                    num_points,
                    border,
                    vi + 1
                );
                render_grid(&grid, &file, &title);
            } else {
                eprintln!("WFC-Amended failed: param set {} variation {}", pi + 1, vi + 1);
            }
        }
        println!("  param set {}/10 done", pi + 1);
    }
    } // end amended

    // ── 4c. WFC Amended, arbitrary borders (1000 images) ──────
    // 1000 images: first 500 keep a fixed thin border while sweeping grid size,
    // point count and wall weight; second 500 sweep the border thickness itself.
    if run_all || borders_only {
    fs::create_dir_all("output/wfc_ammended_borders").unwrap();
    println!("Generating WFC Amended Borders (1000 images)...");

    let grids = [(10usize, 6usize), (14, 8), (18, 9), (20, 10), (28, 15)];
    let weights = [1.0f64, 2.0];
    let variations = 10;
    let mut count = 0usize;

    // First 500 — fixed border thickness, varying everything else.
    let point_opts = [4usize, 6, 8, 10, 12];
    let fixed_border = 1usize;
    for (gi, &(wfc_c, wfc_r)) in grids.iter().enumerate() {
        for (wi, &ew) in weights.iter().enumerate() {
            for (ni, &num_points) in point_opts.iter().enumerate() {
                let tiles = wfc_tiles(ew, 1.0);
                for vi in 0..variations {
                    let base = 1_000_000
                        + (gi * 10_000 + wi * 1_000 + ni * 100 + vi) as u64;
                    let mut wall_grid = None;
                    for attempt in 0..50u64 {
                        let mut rng = StdRng::seed_from_u64(base * 100 + attempt);
                        if let Some(result) = wfc_generate(wfc_c, wfc_r, &tiles, &mut rng) {
                            wall_grid = Some(wfc_to_grid(&result, &tiles));
                            break;
                        }
                    }
                    if let Some(wg) = wall_grid {
                        let mut rng = StdRng::seed_from_u64(base * 7 + 3);
                        let grid = wfc_amended(&wg, num_points, fixed_border, &mut rng);
                        count += 1;
                        let file =
                            format!("output/wfc_ammended_borders/a_fixed_{:04}.png", count);
                        let title = format!(
                            "Border(fixed={}) {}x{} pts={} ew={:.0} #{}",
                            fixed_border, wfc_c, wfc_r, num_points, ew, vi + 1
                        );
                        render_grid(&grid, &file, &title);
                    }
                }
            }
        }
    }
    println!("  first 500 (fixed border) done: {} images", count);

    // Second 500 — sweep the border thickness.
    let border_opts = [1usize, 2, 3, 4, 5];
    let fixed_points = 8usize;
    let mut count2 = 0usize;
    for (gi, &(wfc_c, wfc_r)) in grids.iter().enumerate() {
        for (wi, &ew) in weights.iter().enumerate() {
            for (bi, &border) in border_opts.iter().enumerate() {
                let tiles = wfc_tiles(ew, 1.0);
                for vi in 0..variations {
                    let base = 2_000_000
                        + (gi * 10_000 + wi * 1_000 + bi * 100 + vi) as u64;
                    let mut wall_grid = None;
                    for attempt in 0..50u64 {
                        let mut rng = StdRng::seed_from_u64(base * 100 + attempt);
                        if let Some(result) = wfc_generate(wfc_c, wfc_r, &tiles, &mut rng) {
                            wall_grid = Some(wfc_to_grid(&result, &tiles));
                            break;
                        }
                    }
                    if let Some(wg) = wall_grid {
                        let mut rng = StdRng::seed_from_u64(base * 7 + 3);
                        let grid = wfc_amended(&wg, fixed_points, border, &mut rng);
                        count2 += 1;
                        let file =
                            format!("output/wfc_ammended_borders/b_varborder_{:04}.png", count2);
                        let title = format!(
                            "Border(size={}) {}x{} pts={} ew={:.0} #{}",
                            border, wfc_c, wfc_r, fixed_points, ew, vi + 1
                        );
                        render_grid(&grid, &file, &title);
                    }
                }
            }
        }
    }
    println!("  second 500 (varying border) done: {} images", count2);
    }

    if amended_only {
        println!("Done! 100 amended images regenerated in output/wfc_ammended/");
    } else if borders_only {
        println!("Done! 1000 border images generated in output/wfc_ammended_borders/");
    } else {
        println!("Done! 1500 images generated in output/");
    }
}
