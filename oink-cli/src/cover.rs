//! The cover picture through ratatui-image: Kitty, iTerm2, or Sixel when the
//! terminal has one of them, Unicode half blocks otherwise.

use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};

pub struct Cover {
    protocol: StatefulProtocol,
}

impl Cover {
    /// Ask the terminal which protocol it speaks and prepare `picture`.
    /// Call this after the alternate screen opens and before any event is
    /// read: the probe writes escape codes and reads the reply.
    pub fn new(picture: DynamicImage) -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        Self {
            protocol: picker.new_resize_protocol(picture),
        }
    }

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
