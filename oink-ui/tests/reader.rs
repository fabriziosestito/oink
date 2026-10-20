mod support;

use oink_ui::{draw_empty, Metrics, Theme, Zones};

#[test]
fn the_empty_reader_draws_the_three_zones() {
    let metrics = Metrics::new(320, 240, 28, 36);
    let zones = Zones::from_metrics(metrics);
    let theme = Theme::default();
    let mut framebuffer = support::Framebuffer::<320, 240>::new();

    draw_empty(&mut framebuffer, metrics, theme).unwrap();

    assert_eq!(zones.top.size.height, 28);
    assert_eq!(zones.center.size.height, 176);
    assert_eq!(zones.bottom.size.height, 36);

    assert_eq!(framebuffer.pixel(0, 0), theme.paper);
    assert_eq!(framebuffer.pixel(0, 27), theme.dim);
    assert_eq!(framebuffer.pixel(0, 28), theme.paper);
    assert_eq!(framebuffer.pixel(0, 203), theme.paper);
    assert_eq!(framebuffer.pixel(0, 204), theme.dim);
    assert_eq!(framebuffer.pixel(0, 239), theme.paper);
}

#[test]
fn the_snapshot_is_written_as_a_pgm() {
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    draw_empty(
        &mut framebuffer,
        Metrics::new(320, 240, 28, 36),
        Theme::default(),
    )
    .unwrap();

    let path = framebuffer.write_pgm("empty-reader").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"P5\n320 240\n15\n"));
    assert_eq!(bytes.len(), "P5\n320 240\n15\n".len() + 320 * 240);
}
