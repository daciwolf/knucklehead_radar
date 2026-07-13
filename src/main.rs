use faer::prelude::*;
use rayon::prelude::*; 
use statrs::statistics::Statistics: 
use plotters::prelude::*; 

// this is the x and y lens of the robomaster field
// for the purposes of this library we will treat it like a cartesian field where the bottom left
// is (0,0); 
let RMF: f64 = [28, 15]; 

let RMF_POINTS = mat! [[0.0,0.0,28.0,28.0], [0.0, 15.0, 15.0, 0.0 ]; // define a ploys as thier points
                                                             // going clockwise 


                                                              

//poly_points is designed in the [[[x,y],[x,y]]] format for all code 
//different from rmf points because it works better with plotters
//it is many polys of points that is why it is a triple nexted vector, group -> polys -> points

fn draw_arena(polys_points: vec!, map:  &BitMapBackend) -> Result<(), Box<dyn std::error::Error>> {
    for points in &polys_points{
        root.draw(&Polygon::new(points, BLACK.filled()))?; 
    }
}







fn main(){

    //just drawing the blank 


    let map = BitMapBackend::new("RMF_ARENA.png", (600,400)).into_drawing_area(); 
    let

}

