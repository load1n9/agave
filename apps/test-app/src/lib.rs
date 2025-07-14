use agave_lib::{
    clear_screen, draw_circle, draw_line, draw_rectangle, draw_triangle, fill_circle,
    fill_rectangle, get_dimensions, get_time_ms, Position, RGBA,
};

static mut LAST_TIME: u64 = 0;
static mut FRAME_COUNT: u32 = 0;
static mut ANIMATION_PHASE: f32 = 0.0;

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    unsafe {
        let current_time = get_time_ms();
        FRAME_COUNT += 1;
        
        // Calculate FPS every second
        if current_time - LAST_TIME >= 1000 {
            // We could display FPS here if we had text rendering
            LAST_TIME = current_time;
            FRAME_COUNT = 0;
        }
        
        // Update animation
        ANIMATION_PHASE += 0.05;
        if ANIMATION_PHASE > 6.28 { // 2π
            ANIMATION_PHASE = 0.0;
        }
    }

    let dim = get_dimensions();
    
    // Clear screen with a gradient-like effect
    clear_screen(RGBA::new(10, 10, 20, 255));
    
    // Draw animated background pattern
    unsafe {
        let time_offset = (ANIMATION_PHASE * 100.0) as i32;
        for i in (0..dim.width).step_by(50) {
            let x = (i + time_offset) % dim.width;
            draw_line(
                Position::new(x, 0),
                Position::new(x, dim.height),
                RGBA::new(30, 30, 50, 100),
            );
        }
        
        for i in (0..dim.height).step_by(50) {
            let y = (i + time_offset / 2) % dim.height;
            draw_line(
                Position::new(0, y),
                Position::new(dim.width, y),
                RGBA::new(30, 30, 50, 100),
            );
        }
    }
    
    // Draw interactive elements following mouse
    let mouse_pos = Position::new(mouse_x, mouse_y);
    
    // Main circle following mouse with color based on position
    let red_intensity = ((mouse_x as f32 / dim.width as f32) * 255.0) as i32;
    let blue_intensity = ((mouse_y as f32 / dim.height as f32) * 255.0) as i32;
    
    fill_circle(
        mouse_pos,
        20,
        RGBA::new(red_intensity, 100, blue_intensity, 200),
    );
    
    // Trail effect - smaller circles behind mouse
    for i in 1..=5 {
        let trail_radius = 20 - i * 3;
        let trail_alpha = 200 - i * 30;
        let trail_pos = Position::new(
            mouse_x - i * 5,
            mouse_y - i * 2,
        );
        
        fill_circle(
            trail_pos,
            trail_radius,
            RGBA::new(red_intensity / 2, 50, blue_intensity / 2, trail_alpha),
        );
    }
    
    // Draw animated geometric shapes
    unsafe {
        let center_x = dim.width / 2;
        let center_y = dim.height / 2;
        let radius = 100.0;
        
        // Rotating triangle
        let angle1 = ANIMATION_PHASE;
        let angle2 = ANIMATION_PHASE + 2.09; // 2π/3
        let angle3 = ANIMATION_PHASE + 4.19; // 4π/3
        
        let p1 = Position::new(
            center_x + (radius * angle1.cos()) as i32,
            center_y + (radius * angle1.sin()) as i32,
        );
        let p2 = Position::new(
            center_x + (radius * angle2.cos()) as i32,
            center_y + (radius * angle2.sin()) as i32,
        );
        let p3 = Position::new(
            center_x + (radius * angle3.cos()) as i32,
            center_y + (radius * angle3.sin()) as i32,
        );
        
        draw_triangle(p1, p2, p3, RGBA::new(255, 200, 100, 255));
        
        // Pulsing rectangles in corners
        let pulse = (ANIMATION_PHASE.sin().abs() * 50.0) as i32;
        
        // Top-left
        fill_rectangle(
            Position::new(10, 10),
            30 + pulse,
            30 + pulse,
            RGBA::GREEN,
        );
        
        // Top-right
        fill_rectangle(
            Position::new(dim.width - 40 - pulse, 10),
            30 + pulse,
            30 + pulse,
            RGBA::RED,
        );
        
        // Bottom-left
        fill_rectangle(
            Position::new(10, dim.height - 40 - pulse),
            30 + pulse,
            30 + pulse,
            RGBA::BLUE,
        );
        
        // Bottom-right
        fill_rectangle(
            Position::new(dim.width - 40 - pulse, dim.height - 40 - pulse),
            30 + pulse,
            30 + pulse,
            RGBA::WHITE,
        );
    }
    
    // Draw border
    draw_rectangle(
        Position::new(0, 0),
        dim.width - 1,
        dim.height - 1,
        RGBA::new(100, 100, 100, 255),
    );
    
    // Draw connection line from center to mouse
    draw_line(
        Position::new(dim.width / 2, dim.height / 2),
        mouse_pos,
        RGBA::new(255, 255, 255, 100),
    );
}
