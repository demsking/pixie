use gtk4::cairo;
use gtk4::prelude::*;
use image::DynamicImage;
use pixie_core::PixelRect;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropHandle {
  Top,
  Bottom,
  Left,
  Right,
  TopLeft,
  TopRight,
  BottomLeft,
  BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CropInteractionState {
  Idle,
  Creating,
  Moving,
  Resizing(CropHandle),
  Confirming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropMode {
  Free,
  Original,
  Square11,
  Ratio54,
  Ratio43,
  Ratio32,
  Ratio169,
  Circle,
}

pub struct CanvasState {
  pub image: Option<DynamicImage>,
  pub surface: Option<cairo::ImageSurface>,

  // Crop Tool state
  pub crop_active: bool,
  pub crop_mode: CropMode,
  pub crop_portrait: bool,
  pub crop_rect: Option<PixelRect>,
  pub rule_of_thirds: bool,
  pub interaction_state: CropInteractionState,
  pub drag_start_rect: Option<PixelRect>,
  pub drag_start_point: (f64, f64), // Widget coordinates

  // Zoom state
  pub zoom_factor: Option<f64>,
}

pub struct PixieCanvas {
  pub drawing_area: gtk4::DrawingArea,
  pub state: Rc<RefCell<CanvasState>>,
}

const ZOOM_LEVELS: &[f64] = &[
  0.05, 0.1, 0.2, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 15.0, 20.0,
];

impl PixieCanvas {
  pub fn new() -> Self {
    let drawing_area = gtk4::DrawingArea::builder()
      .hexpand(true)
      .vexpand(true)
      .css_classes(["canvas-surface"])
      .build();

    let state = Rc::new(RefCell::new(CanvasState {
      image: None,
      surface: None,
      crop_active: false,
      crop_mode: CropMode::Free,
      crop_portrait: false,
      crop_rect: None,
      rule_of_thirds: true,
      interaction_state: CropInteractionState::Idle,
      drag_start_rect: None,
      drag_start_point: (0.0, 0.0),
      zoom_factor: None,
    }));

    let state_clone = state.clone();
    drawing_area.set_draw_func(move |area, cr, width, height| {
      Self::draw(area, cr, width, height, &state_clone);
    });

    let canvas = Self {
      drawing_area,
      state,
    };
    canvas.setup_gestures();
    canvas
  }

  pub fn set_image(&self, img: DynamicImage) -> Result<(), String> {
    let surface = dynamic_image_to_surface(&img)?;
    let mut state = self.state.borrow_mut();
    state.image = Some(img);
    state.surface = Some(surface);
    state.crop_rect = None; // Reset crop
    state.interaction_state = CropInteractionState::Idle;
    drop(state);
    self.update_size();
    self.drawing_area.queue_resize();
    self.drawing_area.queue_draw();
    Ok(())
  }

  pub fn set_zoom(&self, zoom: Option<f64>) {
    let mut state = self.state.borrow_mut();
    state.zoom_factor = zoom;
    drop(state);
    self.update_size();
    self.drawing_area.queue_resize();
    self.drawing_area.queue_draw();
  }

  pub fn update_size(&self) {
    let state = self.state.borrow();
    if let Some(ref img) = state.image {
      let img_w = img.width() as f64;
      let img_h = img.height() as f64;
      if let Some(zoom) = state.zoom_factor {
        let w = (img_w * zoom).round() as i32;
        let h = (img_h * zoom).round() as i32;
        self.drawing_area.set_content_width(w);
        self.drawing_area.set_content_height(h);
      } else {
        self.drawing_area.set_content_width(-1);
        self.drawing_area.set_content_height(-1);
      }
    } else {
      self.drawing_area.set_content_width(-1);
      self.drawing_area.set_content_height(-1);
    }
  }

  pub fn get_current_scale(&self) -> f64 {
    let state = self.state.borrow();
    if let Some(zoom) = state.zoom_factor {
      zoom
    } else if let Some(ref img) = state.image {
      let (width, height) = if let Some(parent) = self.drawing_area.parent() {
        (parent.width() as f64, parent.height() as f64)
      } else {
        (
          self.drawing_area.width() as f64,
          self.drawing_area.height() as f64,
        )
      };
      let img_w = img.width() as f64;
      let img_h = img.height() as f64;
      let scale_w = width / img_w;
      let scale_h = height / img_h;
      scale_w.min(scale_h).min(1.0)
    } else {
      1.0
    }
  }

  pub fn zoom_in(&self) -> f64 {
    let current = self.get_current_scale();
    let new_zoom = ZOOM_LEVELS
      .iter()
      .copied()
      .find(|&z| z > current + 0.001)
      .unwrap_or(ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);
    self.set_zoom(Some(new_zoom));
    new_zoom
  }

  pub fn zoom_out(&self) -> f64 {
    let current = self.get_current_scale();
    let new_zoom = ZOOM_LEVELS
      .iter()
      .copied()
      .rev()
      .find(|&z| z < current - 0.001)
      .unwrap_or(ZOOM_LEVELS[0]);
    self.set_zoom(Some(new_zoom));
    new_zoom
  }

  pub fn set_crop_active(&self, active: bool) {
    let mut state = self.state.borrow_mut();
    state.crop_active = active;
    if active && state.crop_rect.is_none() {
      if let Some(ref img) = state.image {
        // Initialize default crop to 80% of image size centered
        let w = img.width();
        let h = img.height();
        let cw = ((w as f64) * 0.8) as u32;
        let ch = ((h as f64) * 0.8) as u32;
        state.crop_rect = Some(PixelRect::new((w - cw) / 2, (h - ch) / 2, cw, ch));
      }
    }
    drop(state);
    self.drawing_area.set_cursor(None);
    self.drawing_area.queue_draw();
  }

  #[allow(dead_code)]
  pub fn set_crop_mode(&self, mode: CropMode) {
    let mut state = self.state.borrow_mut();
    state.crop_mode = mode;
    if let Some(ref img) = state.image {
      let w = img.width();
      let h = img.height();
      // Adjust current crop rectangle to fit the aspect ratio
      if let Some(rect) = state.crop_rect {
        state.crop_rect = Some(apply_ratio_to_rect(rect, mode, w, h, state.crop_portrait));
      }
    }
    drop(state);
    self.drawing_area.queue_draw();
  }

  #[allow(dead_code)]
  pub fn set_rule_of_thirds(&self, show: bool) {
    let mut state = self.state.borrow_mut();
    state.rule_of_thirds = show;
    drop(state);
    self.drawing_area.queue_draw();
  }

  pub fn get_layout_scale_offset(
    area: &gtk4::DrawingArea,
    state: &CanvasState,
  ) -> Option<(f64, f64, f64, f64, f64)> {
    let img = state.image.as_ref()?;
    let img_w = img.width() as f64;
    let img_h = img.height() as f64;

    let (parent_w, parent_h) = if let Some(parent) = area.parent() {
      (parent.width() as f64, parent.height() as f64)
    } else {
      (area.width() as f64, area.height() as f64)
    };
    let area_w = area.width() as f64;
    let area_h = area.height() as f64;

    let (w_f, h_f) = if state.zoom_factor.is_none() {
      (parent_w, parent_h)
    } else {
      (area_w, area_h)
    };

    let scale = if let Some(zoom) = state.zoom_factor {
      zoom
    } else {
      let scale_w = w_f / img_w;
      let scale_h = h_f / img_h;
      scale_w.min(scale_h).min(1.0)
    };

    let ox = if img_w * scale < w_f {
      (w_f - img_w * scale) / 2.0
    } else {
      0.0
    };
    let oy = if img_h * scale < h_f {
      (h_f - img_h * scale) / 2.0
    } else {
      0.0
    };

    Some((scale, ox, oy, img_w, img_h))
  }

  fn draw(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    _width: i32,
    _height: i32,
    state_rc: &Rc<RefCell<CanvasState>>,
  ) {
    let state = state_rc.borrow();
    let surface = match &state.surface {
      Some(s) => s,
      None => {
        return;
      }
    };

    let (scale, ox, oy, img_w, img_h) = match Self::get_layout_scale_offset(_area, &state) {
      Some(layout) => layout,
      None => return,
    };

    cr.save().unwrap();
    cr.translate(ox, oy);
    cr.scale(scale, scale);

    // Draw image surface
    cr.set_source_surface(surface, 0.0, 0.0).unwrap();
    cr.paint().unwrap();
    cr.restore().unwrap();

    // Draw Crop Overlay
    if state.crop_active {
      if let Some(rect) = state.crop_rect {
        let rx = ox + f64::from(rect.x) * scale;
        let ry = oy + f64::from(rect.y) * scale;
        let rw = f64::from(rect.width) * scale;
        let rh = f64::from(rect.height) * scale;

        // 1. Draw dimmed background outside crop rect
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);

        // Top block
        cr.rectangle(ox, oy, img_w * scale, ry - oy);
        // Bottom block
        cr.rectangle(ox, ry + rh, img_w * scale, oy + img_h * scale - (ry + rh));
        // Left block
        cr.rectangle(ox, ry, rx - ox, rh);
        // Right block
        cr.rectangle(rx + rw, ry, ox + img_w * scale - (rx + rw), rh);
        cr.fill().unwrap();

        // 2. Draw crop border
        // First draw a dark contrast shadow line
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.set_line_width(4.0);
        if state.crop_mode == CropMode::Circle {
          cr.arc(
            rx + rw / 2.0,
            ry + rh / 2.0,
            rw / 2.0,
            0.0,
            2.0 * std::f64::consts::PI,
          );
        } else {
          cr.rectangle(rx, ry, rw, rh);
        }
        cr.stroke().unwrap();

        // Draw the main white border on top
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.set_line_width(2.0);
        if state.crop_mode == CropMode::Circle {
          cr.arc(
            rx + rw / 2.0,
            ry + rh / 2.0,
            rw / 2.0,
            0.0,
            2.0 * std::f64::consts::PI,
          );
        } else {
          cr.rectangle(rx, ry, rw, rh);
        }
        cr.stroke().unwrap();

        // 3. Draw Rule of Thirds grid
        if state.rule_of_thirds && state.crop_mode != CropMode::Circle {
          let draw_grid_lines = |ctx: &cairo::Context| {
            // Vertical lines
            ctx.move_to(rx + rw / 3.0, ry);
            ctx.line_to(rx + rw / 3.0, ry + rh);
            ctx.move_to(rx + (2.0 * rw) / 3.0, ry);
            ctx.line_to(rx + (2.0 * rw) / 3.0, ry + rh);
            // Horizontal lines
            ctx.move_to(rx, ry + rh / 3.0);
            ctx.line_to(rx + rw, ry + rh / 3.0);
            ctx.move_to(rx, ry + (2.0 * rh) / 3.0);
            ctx.line_to(rx + rw, ry + (2.0 * rh) / 3.0);
          };

          // Draw dark grid lines first for shadow/contrast
          cr.set_source_rgba(0.0, 0.0, 0.0, 0.3);
          cr.set_line_width(3.0);
          draw_grid_lines(cr);
          cr.stroke().unwrap();

          // Draw crisp white grid lines
          cr.set_source_rgba(1.0, 1.0, 1.0, 0.6);
          cr.set_line_width(1.0);
          draw_grid_lines(cr);
          cr.stroke().unwrap();
        }

        // 4. Draw handles with high-contrast outlines
        let hs = 10.0; // handle size
        let draw_handle = |ctx: &cairo::Context, hx: f64, hy: f64| {
          // White fill
          ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
          ctx.rectangle(hx - hs / 2.0, hy - hs / 2.0, hs, hs);
          ctx.fill().unwrap();

          // Dark stroke
          ctx.set_source_rgba(0.1, 0.1, 0.1, 0.8);
          ctx.set_line_width(1.5);
          ctx.rectangle(hx - hs / 2.0, hy - hs / 2.0, hs, hs);
          ctx.stroke().unwrap();
        };

        draw_handle(cr, rx, ry); // TL
        draw_handle(cr, rx + rw, ry); // TR
        draw_handle(cr, rx, ry + rh); // BL
        draw_handle(cr, rx + rw, ry + rh); // BR

        if state.crop_mode != CropMode::Circle {
          draw_handle(cr, rx + rw / 2.0, ry); // T
          draw_handle(cr, rx + rw / 2.0, ry + rh); // B
          draw_handle(cr, rx, ry + rh / 2.0); // L
          draw_handle(cr, rx + rw, ry + rh / 2.0); // R
        }
      }
    }
  }

  fn setup_gestures(&self) {
    let drag = gtk4::GestureDrag::new();
    let state_clone = self.state.clone();
    let area_clone = self.drawing_area.clone();

    drag.connect_drag_begin(glib::clone!(
      #[weak]
      area_clone,
      move |gesture, x, y| {
        let mut state = state_clone.borrow_mut();
        let rect = match state.crop_rect {
          Some(r) => r,
          None => {
            return;
          }
        };

        let (scale, ox, oy, _, _) = match Self::get_layout_scale_offset(&area_clone, &state) {
          Some(layout) => layout,
          None => return,
        };

        let rx = ox + f64::from(rect.x) * scale;
        let ry = oy + f64::from(rect.y) * scale;
        let rw = f64::from(rect.width) * scale;
        let rh = f64::from(rect.height) * scale;

        let hs = 15.0; // Click tolerance around handles

        let handle = if (x - rx).abs() < hs && (y - ry).abs() < hs {
          Some(CropHandle::TopLeft)
        } else if (x - (rx + rw)).abs() < hs && (y - ry).abs() < hs {
          Some(CropHandle::TopRight)
        } else if (x - rx).abs() < hs && (y - (ry + rh)).abs() < hs {
          Some(CropHandle::BottomLeft)
        } else if (x - (rx + rw)).abs() < hs && (y - (ry + rh)).abs() < hs {
          Some(CropHandle::BottomRight)
        } else if state.crop_mode != CropMode::Circle
          && (x - (rx + rw / 2.0)).abs() < hs
          && (y - ry).abs() < hs
        {
          Some(CropHandle::Top)
        } else if state.crop_mode != CropMode::Circle
          && (x - (rx + rw / 2.0)).abs() < hs
          && (y - (ry + rh)).abs() < hs
        {
          Some(CropHandle::Bottom)
        } else if state.crop_mode != CropMode::Circle
          && (x - rx).abs() < hs
          && (y - (ry + rh / 2.0)).abs() < hs
        {
          Some(CropHandle::Left)
        } else if state.crop_mode != CropMode::Circle
          && (x - (rx + rw)).abs() < hs
          && (y - (ry + rh / 2.0)).abs() < hs
        {
          Some(CropHandle::Right)
        } else {
          None
        };

        state.drag_start_rect = Some(rect);
        state.drag_start_point = (x, y);

        if let Some(h) = handle {
          state.interaction_state = CropInteractionState::Resizing(h);
          gesture.set_state(gtk4::EventSequenceState::Claimed);
        } else if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
          state.interaction_state = CropInteractionState::Moving;
          gesture.set_state(gtk4::EventSequenceState::Claimed);
        } else {
          state.interaction_state = CropInteractionState::Creating;
        }

        update_canvas_cursor(&area_clone, &state, x, y);
      }
    ));

    let state_clone2 = self.state.clone();
    let area_clone2 = self.drawing_area.clone();
    drag.connect_drag_update(move |_gesture, offset_x, offset_y| {
      let mut state = state_clone2.borrow_mut();
      let start_rect = match state.drag_start_rect {
        Some(r) => r,
        None => {
          return;
        }
      };
      let (img_w, img_h) = match &state.image {
        Some(img) => (img.width(), img.height()),
        None => {
          return;
        }
      };

      let (scale, _, _, _, _) = match Self::get_layout_scale_offset(&area_clone2, &state) {
        Some(layout) => layout,
        None => return,
      };

      // Convert offset from widget scale to original image coordinates
      let dx = (offset_x / scale).round() as i32;
      let dy = (offset_y / scale).round() as i32;

      let mut rect = start_rect;

      match state.interaction_state {
        CropInteractionState::Moving => {
          let new_x = ((start_rect.x as i32) + dx).clamp(0, (img_w - rect.width) as i32) as u32;
          let new_y = ((start_rect.y as i32) + dy).clamp(0, (img_h - rect.height) as i32) as u32;
          rect.x = new_x;
          rect.y = new_y;
        }
        CropInteractionState::Resizing(handle) => {
          let min_size = 20i32;
          let mode = state.crop_mode;

          match handle {
            CropHandle::TopLeft => {
              let max_x = (start_rect.x + start_rect.width).saturating_sub(min_size as u32);
              let max_y = (start_rect.y + start_rect.height).saturating_sub(min_size as u32);
              let new_x = ((start_rect.x as i32) + dx).clamp(0, max_x as i32) as u32;
              let new_y = ((start_rect.y as i32) + dy).clamp(0, max_y as i32) as u32;

              rect.width = (start_rect.x + start_rect.width).saturating_sub(new_x);
              rect.height = (start_rect.y + start_rect.height).saturating_sub(new_y);
              rect.x = new_x;
              rect.y = new_y;
            }
            CropHandle::TopRight => {
              let max_y = (start_rect.y + start_rect.height).saturating_sub(min_size as u32);
              let new_y = ((start_rect.y as i32) + dy).clamp(0, max_y as i32) as u32;
              let new_w = ((start_rect.width as i32) + dx)
                .max(min_size)
                .clamp(0, (img_w - start_rect.x) as i32) as u32;

              rect.height = (start_rect.y + start_rect.height).saturating_sub(new_y);
              rect.y = new_y;
              rect.width = new_w;
            }
            CropHandle::BottomLeft => {
              let max_x = (start_rect.x + start_rect.width).saturating_sub(min_size as u32);
              let new_x = ((start_rect.x as i32) + dx).clamp(0, max_x as i32) as u32;
              let new_h = ((start_rect.height as i32) + dy)
                .max(min_size)
                .clamp(0, (img_h - start_rect.y) as i32) as u32;

              rect.width = (start_rect.x + start_rect.width).saturating_sub(new_x);
              rect.x = new_x;
              rect.height = new_h;
            }
            CropHandle::BottomRight => {
              let new_w = ((start_rect.width as i32) + dx)
                .max(min_size)
                .clamp(0, (img_w - start_rect.x) as i32) as u32;
              let new_h = ((start_rect.height as i32) + dy)
                .max(min_size)
                .clamp(0, (img_h - start_rect.y) as i32) as u32;
              rect.width = new_w;
              rect.height = new_h;
            }
            CropHandle::Top => {
              let max_y = (start_rect.y + start_rect.height).saturating_sub(min_size as u32);
              let new_y = ((start_rect.y as i32) + dy).clamp(0, max_y as i32) as u32;
              rect.height = (start_rect.y + start_rect.height).saturating_sub(new_y);
              rect.y = new_y;
            }
            CropHandle::Bottom => {
              let new_h = ((start_rect.height as i32) + dy)
                .max(min_size)
                .clamp(0, (img_h - start_rect.y) as i32) as u32;
              rect.height = new_h;
            }
            CropHandle::Left => {
              let max_x = (start_rect.x + start_rect.width).saturating_sub(min_size as u32);
              let new_x = ((start_rect.x as i32) + dx).clamp(0, max_x as i32) as u32;
              rect.width = (start_rect.x + start_rect.width).saturating_sub(new_x);
              rect.x = new_x;
            }
            CropHandle::Right => {
              let new_w = ((start_rect.width as i32) + dx)
                .max(min_size)
                .clamp(0, (img_w - start_rect.x) as i32) as u32;
              rect.width = new_w;
            }
          }

          rect = apply_ratio_to_rect(rect, mode, img_w, img_h, state.crop_portrait);
        }
        _ => {}
      }

      state.crop_rect = Some(rect);

      // Update cursor during drag
      let start_point = state.drag_start_point;
      let x = start_point.0 + offset_x;
      let y = start_point.1 + offset_y;
      update_canvas_cursor(&area_clone2, &state, x, y);

      drop(state);
      area_clone2.queue_draw();
    });

    let state_clone3 = self.state.clone();
    let area_clone3 = self.drawing_area.clone();
    drag.connect_drag_end(move |_, offset_x, offset_y| {
      let mut state = state_clone3.borrow_mut();
      state.interaction_state = CropInteractionState::Idle;
      state.drag_start_rect = None;

      let start_point = state.drag_start_point;
      let x = start_point.0 + offset_x;
      let y = start_point.1 + offset_y;
      update_canvas_cursor(&area_clone3, &state, x, y);

      drop(state);
      area_clone3.queue_draw();
    });

    self.drawing_area.add_controller(drag);

    // Setup mouse hover motion events to update cursor
    let motion = gtk4::EventControllerMotion::new();
    let state_motion = self.state.clone();
    let area_motion = self.drawing_area.clone();
    motion.connect_motion(move |_controller, x, y| {
      let state = state_motion.borrow();
      update_canvas_cursor(&area_motion, &state, x, y);
    });

    let area_leave = self.drawing_area.clone();
    motion.connect_leave(move |_controller| {
      area_leave.set_cursor(None);
    });

    self.drawing_area.add_controller(motion);
  }
}

// Helper to dynamically update the mouse cursor during crop interaction or hover
fn update_canvas_cursor(area: &gtk4::DrawingArea, state: &CanvasState, x: f64, y: f64) {
  if !state.crop_active {
    area.set_cursor(None);
    return;
  }

  let cursor_name = match state.interaction_state {
    CropInteractionState::Creating => Some("crosshair"),
    CropInteractionState::Moving => Some("move"),
    CropInteractionState::Resizing(handle) => match handle {
      CropHandle::TopLeft => Some("nw-resize"),
      CropHandle::Top => Some("n-resize"),
      CropHandle::TopRight => Some("ne-resize"),
      CropHandle::Right => Some("e-resize"),
      CropHandle::BottomRight => Some("se-resize"),
      CropHandle::Bottom => Some("s-resize"),
      CropHandle::BottomLeft => Some("sw-resize"),
      CropHandle::Left => Some("w-resize"),
    },
    CropInteractionState::Idle | CropInteractionState::Confirming => {
      if let Some(rect) = state.crop_rect {
        if let Some((scale, ox, oy, _, _)) = PixieCanvas::get_layout_scale_offset(area, state) {
          let rx = ox + f64::from(rect.x) * scale;
          let ry = oy + f64::from(rect.y) * scale;
          let rw = f64::from(rect.width) * scale;
          let rh = f64::from(rect.height) * scale;

          let hs = 15.0; // handle size tolerance

          if (x - rx).abs() < hs && (y - ry).abs() < hs {
            Some("nw-resize")
          } else if (x - (rx + rw)).abs() < hs && (y - ry).abs() < hs {
            Some("ne-resize")
          } else if (x - rx).abs() < hs && (y - (ry + rh)).abs() < hs {
            Some("sw-resize")
          } else if (x - (rx + rw)).abs() < hs && (y - (ry + rh)).abs() < hs {
            Some("se-resize")
          } else if state.crop_mode != CropMode::Circle
            && (x - (rx + rw / 2.0)).abs() < hs
            && (y - ry).abs() < hs
          {
            Some("n-resize")
          } else if state.crop_mode != CropMode::Circle
            && (x - (rx + rw / 2.0)).abs() < hs
            && (y - (ry + rh)).abs() < hs
          {
            Some("s-resize")
          } else if state.crop_mode != CropMode::Circle
            && (x - rx).abs() < hs
            && (y - (ry + rh / 2.0)).abs() < hs
          {
            Some("w-resize")
          } else if state.crop_mode != CropMode::Circle
            && (x - (rx + rw)).abs() < hs
            && (y - (ry + rh / 2.0)).abs() < hs
          {
            Some("e-resize")
          } else if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
            Some("move")
          } else {
            Some("crosshair")
          }
        } else {
          None
        }
      } else {
        None
      }
    }
  };

  if let Some(name) = cursor_name {
    if let Some(cursor) = gtk4::gdk::Cursor::from_name(name, None) {
      area.set_cursor(Some(&cursor));
    } else {
      area.set_cursor(None);
    }
  } else {
    area.set_cursor(None);
  }
}

// Helper to convert dynamic image to Cairo surface
fn dynamic_image_to_surface(img: &DynamicImage) -> Result<cairo::ImageSurface, String> {
  let rgba = img.to_rgba8();
  let (w, h) = (rgba.width() as i32, rgba.height() as i32);
  let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h)
    .map_err(|e| format!("Failed to create drawing surface for {w}x{h} image: {e}"))?;
  {
    let mut data = surface
      .data()
      .map_err(|e| format!("Failed to access drawing surface memory: {e}"))?;
    let bytes = rgba.as_raw();
    for i in 0..(w * h) as usize {
      let r = bytes[i * 4];
      let g = bytes[i * 4 + 1];
      let b = bytes[i * 4 + 2];
      let a = bytes[i * 4 + 3];
      // Cairo's ARgb32 expects premultiplied alpha in BGRA byte order.
      let pb = ((u16::from(b) * u16::from(a) + 127) / 255) as u8;
      let pg = ((u16::from(g) * u16::from(a) + 127) / 255) as u8;
      let pr = ((u16::from(r) * u16::from(a) + 127) / 255) as u8;
      data[i * 4] = pb;
      data[i * 4 + 1] = pg;
      data[i * 4 + 2] = pr;
      data[i * 4 + 3] = a;
    }
  }
  Ok(surface)
}

// Adjusts the width/height of a rect to match the aspect ratio of the selected CropMode
pub(super) fn apply_ratio_to_rect(
  mut rect: PixelRect,
  mode: CropMode,
  img_w: u32,
  img_h: u32,
  portrait: bool,
) -> PixelRect {
  let base_ratio = match mode {
    CropMode::Free => {
      return rect;
    }
    CropMode::Original => f64::from(img_w).max(1.0) / f64::from(img_h).max(1.0),
    CropMode::Square11 | CropMode::Circle => 1.0,
    CropMode::Ratio54 => 5.0 / 4.0,
    CropMode::Ratio43 => 4.0 / 3.0,
    CropMode::Ratio32 => 3.0 / 2.0,
    CropMode::Ratio169 => 16.0 / 9.0,
  };

  let ratio = match mode {
    CropMode::Free | CropMode::Square11 | CropMode::Circle => base_ratio,
    _ if portrait => 1.0 / base_ratio,
    _ => base_ratio,
  };

  // Keep width, adjust height to match ratio, clamp within boundaries
  let target_h = ((rect.width as f64) / ratio).round() as u32;
  if rect.y + target_h <= img_h {
    rect.height = target_h;
  } else {
    // If height exceeds, scale down width instead
    rect.height = img_h - rect.y;
    rect.width = ((rect.height as f64) * ratio).round() as u32;
  }

  rect
}
