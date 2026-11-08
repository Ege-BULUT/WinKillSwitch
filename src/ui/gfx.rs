//! The glass compositor. Each `rune::effect(kind)` slot in the view becomes one rune-fx layer:
//! the background mesh gradient, liquid-glass panels and the tab bar, or fluted glass. Colours come
//! from the current palette in app state; the slot's four floats say dark/light, strength and role.

use rune::compose::{Compositor, Frame};
use rune::render::{EffectPhase, EffectSite, KIND_EFFECT};
use rune_fx::*;

use crate::app::App;
use crate::model::Appearance;
use crate::ui::palettes;

pub const MESH: u32 = 1;
pub const LENS: u32 = 2;
pub const FLUTED: u32 = 3;

/// Roles a lens slot plays (effect data [2]).
pub const ROLE_PANEL: f32 = 0.0;
pub const ROLE_BAR: f32 = 1.0;
pub const ROLE_DROP: f32 = 2.0;

/// Effect data for a slot: whether the frame is dark, the glass strength and the role.
pub fn data(dark: bool, strength: u8, role: f32) -> [f32; 4] {
    [if dark { 1.0 } else { 0.0 }, strength as f32, role, 0.0]
}

fn layer(kind: u32, d: [f32; 4], shape: Shape, t: f32, a: &Appearance) -> Option<FxLayer> {
    let dark = d[0] > 0.5;
    let strength = d[1].clamp(0.0, 3.5);
    let pal = palettes::find(&a.palette);
    let v = pal.variant(dark);
    // How far glass leans toward the palette surface, and how frosted it is.
    let tint_amount = if dark { 0.42 } else { 0.7 } + 0.08 * strength;
    let frost = 0.1 + 0.22 * strength;
    let fx = match kind {
        MESH => {
            let stops = palettes::mesh(v, a.accent, dark);
            let p = MeshGradientParams { colors: Stops::hex(&stops), speed: 0.18, drift: 0.22, ..MeshGradientParams::dusk() };
            return Some(Fx::fill(p).on(shape).at(if a.animated { t } else { 4.0 }).seed(7));
        }
        LENS => {
            let role = d[2];
            let (bevel, depth, mag, frost, tint) = if role == ROLE_DROP {
                (12.0, 16.0, 1.08, 0.05, 0.12)
            } else if role == ROLE_BAR {
                (16.0, 18.0, 1.0, frost * 0.6, tint_amount * 0.8)
            } else {
                (14.0, 10.0, 1.0, frost, tint_amount)
            };
            Fx::lens(LensParams {
                bevel,
                depth,
                dispersion: 0.14,
                magnification: mag,
                frost,
                tint: rgb(v.surface),
                tint_amount: tint,
                ..Default::default()
            })
        }
        FLUTED => Fx::fill(FlutedGlassParams {
            rib: 16.0,
            refraction: 14.0,
            frost: frost * 0.8,
            dispersion: 0.25,
            highlight: 0.5,
            tint: rgb(v.surface),
            tint_amount,
            ..FlutedGlassParams::reeded()
        }),
        _ => return None,
    };
    Some(fx.on(shape).at(t))
}

#[derive(Default)]
pub struct Gfx {
    fx: Option<FxRenderer>,
    animate: bool,
}

impl Compositor<App> for Gfx {
    fn animating(&self) -> bool {
        self.animate
    }

    fn render(&mut self, f: &mut Frame<'_, App>) {
        let (device, queue, size) = (f.device, f.queue, f.size);
        let t = f.rt.t as f32;
        let appearance = f.rt.state.appearance.clone();
        self.animate = appearance.animated;
        let marks = &f.rt.marks;
        let mut at: Vec<u32> = Vec::new();
        let mut layers: Vec<FxLayer> = Vec::new();
        let mut placements: Vec<FxSlot> = Vec::new();
        for (n, i) in f.rt.instances.iter().enumerate() {
            if i.params[1] != KIND_EFFECT {
                continue;
            }
            let kind = marks.get(n).copied().unwrap_or(0) & 0xff;
            let k = i.params[3];
            let [x, y, w, h] = i.rect.map(|v| v / k);
            let r = i.radii.iter().fold(0.0f32, |a, v| a.max(v.abs())) / k;
            let Some(l) = layer(kind, i.uv, Shape::rounded(x, y, w, h, r), t, &appearance) else { continue };
            at.push(n as u32);
            layers.push(l);
            placements.push(FxSlot::default().scale(k).clip(i.clip, i.fade));
        }
        let fx = self.fx.get_or_insert_with(|| FxRenderer::new(device, f.format, FxOptions { dither: true }));
        fx.set_srgb_blend(f.srgb_blend());
        fx.add_format(device, wgpu::TextureFormat::Rgba16Float);
        fx.prepare_with_slots(device, queue, &FxFrame { viewport: [size.0, size.1], scale: 1.0, layers: &layers, backdrop: None, sources: &[] }, &placements);
        let clear = f.rt.clear_color();
        f.ui.prepare(device, queue, &mut f.rt.text.atlas, &f.rt.instances, size);
        f.ui.draw_with_backdrop(f.encoder, f.target, clear, &mut |phase, s: &EffectSite<'_>| {
            let Some(k) = at.iter().position(|n| *n == s.index) else { return };
            match phase {
                EffectPhase::Backdrop(enc) => {
                    if let Some(bd) = s.backdrop {
                        fx.blur_backdrop(device, queue, enc, bd, k);
                    }
                }
                EffectPhase::Draw(pass) => fx.draw_layer_in(pass, s.format, s.origin, k),
            }
        });
    }
}
