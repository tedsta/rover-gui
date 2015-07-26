use conrod::Color;
use graphics::Context;
use opengl_graphics::GlGraphics;
use opengl_graphics::glyph_cache::GlyphCache;

struct Line {
    color: Color,
    points: Vec<(f64, f64)>,
}

impl Line {
    fn new(color: Color, points: Vec<(f64, f64)>) -> Line {
        Line {
            color: color,
            points: points,
        }
    }
}

pub struct LineGraph {
    lines: Vec<Line>,
    pub size: (f64, f64),
    pub x_interval: (f64, f64),
    pub y_interval: (f64, f64),
}

impl LineGraph {
    pub fn new(size: (f64, f64), x_interval: (f64, f64), y_interval: (f64, f64), line_colors: Vec<Color>) -> LineGraph {
        LineGraph {
            lines: line_colors.into_iter().map(|c| Line::new(c, Vec::new())).collect(),
            size: size,
            x_interval: x_interval,
            y_interval: y_interval,
        }
    }
    
    pub fn draw(&self, c: Context, gl: &mut GlGraphics, glyph_cache: &mut GlyphCache) {
        use graphics::*;
        
        Rectangle::new([0.3, 0.3, 1.0, 1.0])
            .draw([0.0, 0.0, self.size.0, self.size.1],
                  &c.draw_state, c.transform,
                  gl);

        {
            // Draw upper scale
            let c = c.trans(2.0, 2.0+12.0);
            Text::colored([1.0; 4], 12).draw(format!("{}", self.y_interval.1).as_str(),
                                             glyph_cache,
                                             &c.draw_state, c.transform,
                                             gl);
        }
        {
            // Draw lower scale
            let c = c.trans(2.0, self.size.1 - 2.0);
            Text::colored([1.0; 4], 12).draw(format!("{}", self.y_interval.0).as_str(),
                                             glyph_cache,
                                             &c.draw_state, c.transform,
                                             gl);
        }
        
        for line in &self.lines {
            for i in (1..line.points.len()) {
                let (x, y) = line.points[i];
                let (last_x, last_y) = line.points[i - 1];
                
                let x_norm = (x - self.x_interval.0)/(self.x_interval.1 - self.x_interval.0);
                let y_norm = (y - self.y_interval.0)/(self.y_interval.1 - self.y_interval.0);
                
                let last_x_norm = (last_x - self.x_interval.0)/(self.x_interval.1 - self.x_interval.0);
                let last_y_norm = (last_y - self.y_interval.0)/(self.y_interval.1 - self.y_interval.0);
                
                if last_x >= self.x_interval.0 && x <= self.x_interval.1 {
                    Line::new([1.0, 0.0, 0.0, 1.0], 1.0)
                        .draw([last_x_norm * self.size.0, self.size.1 - last_y_norm*self.size.1,
                               x_norm * self.size.0, self.size.1 - y_norm*self.size.1],
                              &c.draw_state, c.transform,
                              gl);
                }
            }
        }
    }
    
    pub fn add_point(&mut self, line_index: usize, x: f64, y: f64) {
        let ref mut points = self.lines[line_index].points;
        if points.len() == 0 || points[points.len()-1].0 < x {
            points.push((x, y));
        }
    }
    
    pub fn num_points(&self, line_index: usize) -> usize {
        self.lines[line_index].points.len()
    }
}
