use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
  app.last_frame_area = frame.area();
  let tile_areas = app.layout.resolve(frame.area());

  {
    let App { tiles, ctx, .. } = app;

    for (id, tile) in tiles.iter_mut() {
      if let Some(area) = tile_areas.get(id)
        && tile.visible()
      {
        tile.render(*area, frame, ctx);
      }
    }
  }

  if let Some(ref popup) = app.active_popup {
    popup.render(frame.area(), frame, &app.ctx);
  }
}
