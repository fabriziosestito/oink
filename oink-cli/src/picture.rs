//! Pictures through ratatui-image: Kitty, iTerm2, or Sixel when the
//! terminal has one of them, Unicode half blocks otherwise. The cover and
//! the dialog portraits draw through the same painter.

use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};

/// One picture ready to draw.
pub struct Picture {
    protocol: StatefulProtocol,
}

impl Picture {
    /// Draw the picture centered in `area`, scaled down to fit.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let size = self.protocol.size_for(Resize::Fit(None), area.as_size());
        let width = size.width.min(area.width);
        let height = size.height.min(area.height);
        let centered = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        frame.render_stateful_widget(StatefulImage::default(), centered, &mut self.protocol);
    }
}

/// The terminal probe: which graphics protocol the terminal speaks.
pub struct Painter {
    picker: Picker,
}

impl Painter {
    /// Ask the terminal which protocol it speaks. Call once after the
    /// alternate screen opens and before any event is read: the probe
    /// writes escape codes and reads the reply.
    pub fn probe() -> Self {
        Self {
            picker: Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()),
        }
    }

    /// Prepare `picture` for drawing.
    pub fn view(&self, picture: DynamicImage) -> Picture {
        Picture {
            protocol: self.picker.new_resize_protocol(picture),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn a_picture_draws_into_a_test_terminal() {
        let picture = image::DynamicImage::new_rgb8(4, 4);
        let picker = Picker::halfblocks();
        let mut picture = Picture {
            protocol: picker.new_resize_protocol(picture),
        };
        let mut terminal = Terminal::new(TestBackend::new(10, 6)).unwrap();
        terminal
            .draw(|frame| picture.render(frame, ratatui::layout::Rect::new(0, 0, 10, 6)))
            .unwrap();
    }
}
