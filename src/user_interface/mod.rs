use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use imgui::{self, Context, FontId, Key, StyleColor, StyleVar};
use regex::bytes::Replacer;

use crate::{editor, tools::ToolEnum, tools::EditorEvent};
use crate::editor::Editor;
use glifparser::glif::{LayerOperation};

pub mod icons;
 
// These are before transformation by STATE.dpi (glutin scale_factor)
pub const TOOLBOX_OFFSET_X: f32 = 10.;
pub const TOOLBOX_OFFSET_Y: f32 = TOOLBOX_OFFSET_X;
pub const TOOLBOX_WIDTH: f32 = 55.;
pub const TOOLBOX_HEIGHT: f32 = 300.;

#[derive(Clone)]
pub enum InputPrompt {
    Text {
        label: String,
        default: String,
        func: Rc<dyn Fn(&mut Editor, String)>
    },
    Color {
        label: String,
        default: [f32; 4],
        func: Rc<dyn Fn(&mut Editor, [f32; 4])>
    }
}
pub fn setup_imgui() -> Context {
    let mut imgui = Context::create();
    {
        // Fix incorrect colors with sRGB framebuffer
        fn imgui_gamma_to_linear(col: [f32; 4]) -> [f32; 4] {
            let x = col[0].powf(2.2);
            let y = col[1].powf(2.2);
            let z = col[2].powf(2.2);
            let w = 1.0 - (1.0 - col[3]).powf(2.2);
            [x, y, z, w]
        }

        let style = imgui.style_mut();
        for col in 0..style.colors.len() {
            style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
        }
    }

    imgui.set_ini_filename(None);
    imgui.style_mut().use_light_colors();

    // TODO: Implement proper DPI scaling
    let scale_factor = 1.;
    let font_size = (16.0 * scale_factor) as f32;
    let icon_font_size = (36.0 * scale_factor) as f32;

    let id = imgui.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: &crate::system_fonts::SYSTEMSANS.data,
            size_pixels: font_size,
            config: Some(imgui::FontConfig {
                oversample_h: 3,
                oversample_v: 3,
                glyph_ranges: imgui::FontGlyphRanges::from_slice(&[
                    0x0020 as u16,
                    0x00FF as u16,
                    0x03B8 as u16, // for Greek theta
                    0x03B8 as u16, // for Greek theta
                    0,
                ]),
                ..Default::default()
            }),
        },
        imgui::FontSource::TtfData {
            data: include_bytes!("../../resources/fonts/icons.ttf"),
            size_pixels: icon_font_size,
            config: Some(imgui::FontConfig {
                glyph_ranges: imgui::FontGlyphRanges::from_slice(&[
                    0xF000 as u16,
                    0xF100 as u16,
                    0,
                ]),
                ..Default::default()
            }),
        },
    ]);

    let id1= imgui.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: &crate::system_fonts::SYSTEMSANS.data,
            size_pixels: font_size,
            config: Some(imgui::FontConfig {
                oversample_h: 3,
                oversample_v: 3,
                ..Default::default()
            }),
        },
        imgui::FontSource::TtfData {
            data: include_bytes!("../../resources/fonts/icons.ttf"),
            size_pixels: 28.,
            config: Some(imgui::FontConfig {
                glyph_ranges: imgui::FontGlyphRanges::from_slice(&[
                    0xF000 as u16,
                    0xF100 as u16,
                    0,
                ]),
                ..Default::default()
            }),
        },
    ]);

    FONT_IDS.with(|ids| {
        ids.borrow_mut().push(id);
        ids.borrow_mut().push(id1);
    });
    PROMPT_STR.with(|prompt_str| {
        prompt_str.borrow_mut().reserve(256)
    });
    imgui
}

pub fn build_and_check_button(v: &mut Editor, ui: &imgui::Ui, mode: ToolEnum, icon: &[u8]) {
    let mut pop_me = None;
    if v.get_tool() == mode {
        pop_me = Some(ui.push_style_color(imgui::StyleColor::Button, [0., 0., 0., 0.2]));
    }
    // Icons are always constant so this is not really unsafe.
    ui.button(
        unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icon) },
        [0., 30.],
    );
    if ui.is_item_clicked(imgui::MouseButton::Left) {
        v.set_tool(mode);
    }
    if let Some(p) = pop_me {
        p.pop(ui);
    }
}

pub fn build_and_check_layer_list(v: &mut Editor, ui: &imgui::Ui) {

    let active_layer = v.get_active_layer();

    let pop_me = ui.push_style_color(imgui::StyleColor::Button, [0., 0., 0., 0.2]);

    ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::PLUS) }, [0., 0.]);
    //ui.push_item_width(-0.5);
    if ui.is_item_clicked(imgui::MouseButton::Left) {
        v.new_layer();
    }

    ui.same_line(0.);
    ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::MINUS) }, [0., 0.]);
    ui.push_item_width(-0.5);
    if ui.is_item_clicked(imgui::MouseButton::Left) {
        v.delete_layer(active_layer, true);
    }

    ui.same_line(0.);
    ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::ARROWUP) }, [0., 0.]);
    if ui.is_item_clicked(imgui::MouseButton::Left) {
        if active_layer != 0 {
            let start_layer_type = v.with_glyph(|glif| glif.layers[active_layer].operation.clone());
            let target_layer_type = v.with_glyph(|glif| glif.layers[active_layer-1].operation.clone());

            v.swap_layers(active_layer, active_layer-1, true);
        }
    }

    let layer_count = v.get_layer_count();
    ui.same_line(0.);
    ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::ARROWDOWN) }, [0., 0.]);
    if ui.is_item_clicked(imgui::MouseButton::Left) {
        if active_layer != layer_count-1 {
            v.swap_layers(active_layer, active_layer+1, true);
        }
    }

    pop_me.pop(ui);
    
    ui.separator();

    for layer in 0 .. layer_count {
        let layer_op = v.with_glyph(|glif| glif.layers[layer].operation.clone());
        let layer_temp_name = imgui::im_str!("{0}", v.with_glyph(|glif| { glif.layers[layer].name.clone() }));
        let im_str = imgui::ImString::from(layer_temp_name);

        let font_token = ui.push_font(FONT_IDS.with(|ids| { ids.borrow()[1] }));
        let no_padding = ui.push_style_var(StyleVar::ItemSpacing([0., 0.]));
        let custom_button_color = ui.push_style_color(imgui::StyleColor::Button, ui.style_color(StyleColor::WindowBg));

        if layer_op.is_some() {
            ui.dummy([28., 0.]);
            ui.same_line(0.);
        }
        let layer_visible = v.with_glyph(|glif| glif.layers[layer].visible);
        let eye_con = if layer_visible { icons::OPENEYE } else { icons::CLOSEDEYE };
        ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(eye_con) }, [0., 0.]);
        if ui.is_item_clicked(imgui::MouseButton::Left) {
            let active_layer = v.get_active_layer();
            v.set_active_layer(layer);

            v.begin_layer_modification("Toggled layer visibility.");
            v.with_active_layer_mut(|layer| layer.visible = !layer.visible );
            v.end_layer_modification();

            v.set_active_layer(active_layer);
        }

        ui.same_line(0.);
        ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::RENAME) }, [0., 0.]);
        if ui.is_item_clicked(imgui::MouseButton::Left) {
            v.prompts.push(InputPrompt::Text {
                label: "Layer name:".to_string(),
                default: v.with_glyph(|glif| glif.layers[layer].name.clone()),
                func: Rc::new(move |editor, string| {
                    let active_layer = editor.get_active_layer();
                    editor.set_active_layer(layer);
    
                    editor.begin_layer_modification("Renamed layer.");
                    editor.with_active_layer_mut(|layer| layer.name = string.clone());
                    editor.end_layer_modification();
    
                    editor.set_active_layer(active_layer);
                }),
            });
        }
        ui.same_line(0.);
        
        ui.button(unsafe { imgui::ImStr::from_utf8_with_nul_unchecked(icons::LAYERCOMBINE) }, [0., 0.]);
        if ui.is_item_clicked(imgui::MouseButton::Left) {
            let active_layer = v.get_active_layer();
            v.set_active_layer(layer);
            v.begin_layer_modification("Changed layer operation.");
            v.with_active_layer_mut(|layer| {
                layer.operation = Some(LayerOperation::Difference);
            });
            v.end_layer_modification();
            v.set_active_layer(active_layer);
        }

        if layer_op.is_none() {
            ui.same_line(0.);
            let layer_color = v.with_glyph(|glif| glif.layers[layer].color);
            let color_token = ui.push_style_color(imgui::StyleColor::Button, layer_color);
            ui.button(imgui::im_str!("##"), [0., 0.]);
            if ui.is_item_clicked(imgui::MouseButton::Left) {
                v.prompts.push(InputPrompt::Color {
                    label: "Layer color:".to_string(),
                    default: v.with_glyph(|glif| glif.layers[layer].color.clone()),
                    func: Rc::new(move |editor, color| {
                        let active_layer = editor.get_active_layer();
                        editor.set_active_layer(layer);
        
                        editor.begin_layer_modification("Changed layer color.");
                        editor.with_active_layer_mut(|layer| layer.color = color.clone());
                        editor.end_layer_modification();
        
                        editor.set_active_layer(active_layer);
                    }),
                });
            }
            color_token.pop(ui);
        }

        font_token.pop(ui);
        custom_button_color.pop(ui);

        ui.same_line(0.);
        let mut pop_me = None;
        if active_layer != layer {
            pop_me = Some(ui.push_style_color(imgui::StyleColor::Button, [0., 0., 0., 0.2]));
        }
        ui.button(&im_str, [-1., 0.]);
        if ui.is_item_clicked(imgui::MouseButton::Left) {
            v.set_active_layer(layer);
        }
        if let Some(p) = pop_me {
            p.pop(ui);
        }
        no_padding.pop(ui);
    }
}

pub const LAYERBOX_WIDTH: f32 = 250.;
pub const LAYERBOX_HEIGHT: f32 = 250.;
pub const PROMPTBOX_WIDTH: f32 = 250.;
pub const PROMPTBOX_HEIGHT: f32 = 65.;

pub fn get_tools_dialog_rect(v: &Editor) -> (f32, f32, f32, f32) {
    (
        v.viewport.winsize.0 as f32 - (LAYERBOX_WIDTH) - (TOOLBOX_OFFSET_X),
        v.viewport.winsize.1 as f32 - (LAYERBOX_HEIGHT * 2.) - (TOOLBOX_OFFSET_Y * 2.),
        LAYERBOX_WIDTH,
        LAYERBOX_HEIGHT,
    )
}

pub fn build_imgui_ui(v: &mut Editor, ui: &mut imgui::Ui) {
    imgui::Window::new(imgui::im_str!("Tools"))
        .bg_alpha(1.) // See comment on fn redraw_skia
        .flags(
            #[rustfmt::skip]
                    imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_COLLAPSE,
        )
        .position(
            [TOOLBOX_OFFSET_X, TOOLBOX_OFFSET_Y],
            imgui::Condition::Always,
        )
        .size([TOOLBOX_WIDTH, TOOLBOX_HEIGHT], imgui::Condition::Always)
        .build(ui, || {
            build_and_check_button(v, &ui, ToolEnum::Pan, &icons::PAN);
            build_and_check_button(v, &ui, ToolEnum::Select, &icons::SELECT);
            ui.separator();
            build_and_check_button(v, &ui, ToolEnum::Zoom, &icons::ZOOM);
            ui.separator();
            build_and_check_button(v, &ui, ToolEnum::Anchors, &icons::ANCHOR);
            ui.separator();
            build_and_check_button(v, &ui, ToolEnum::Pen, &icons::PEN);
            build_and_check_button(v, &ui, ToolEnum::VWS, &icons::VWS);
            build_and_check_button(v, &ui, ToolEnum::Shapes, &icons::SHAPES);
        });

    imgui::Window::new( imgui::im_str!("Layers"))
        .bg_alpha(1.)
        .flags(
            #[rustfmt::skip]
                    imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_COLLAPSE
        )
        .position([v.viewport.winsize.0 as f32 - LAYERBOX_WIDTH - TOOLBOX_OFFSET_X , v.viewport.winsize.1 as f32 - TOOLBOX_OFFSET_Y - LAYERBOX_HEIGHT], imgui::Condition::Always)
        .size([LAYERBOX_WIDTH, LAYERBOX_HEIGHT], imgui::Condition::Always)
        .build(ui, || {
            build_and_check_layer_list(v, ui)
        });

    build_and_check_prompts(v, ui);

    v.dispatch_editor_event(EditorEvent::Ui {
        ui: ui
    });
}

fn build_and_check_prompts(v: &mut Editor, ui: &mut imgui::Ui)
{
    if v.prompts.is_empty() { return };

    imgui::Window::new(&imgui::im_str!("##"))
    .flags(
        #[rustfmt::skip]
                imgui::WindowFlags::NO_RESIZE
            | imgui::WindowFlags::NO_MOVE
            | imgui::WindowFlags::NO_COLLAPSE
            | imgui::WindowFlags::NO_DECORATION
            | imgui::WindowFlags::NO_BACKGROUND
    )
    .position([0., 0.], imgui::Condition::Always)
    .size([v.viewport.winsize.0 as f32, v.viewport.winsize.1 as f32], imgui::Condition::Always)
    .build(ui, || {
        ui.invisible_button(&imgui::im_str!("##"), [-1., -1.]);
    });

    match v.prompts[0].clone() {
        InputPrompt::Text{ label, default, func} => {
            imgui::Window::new(&imgui::im_str!("{}", label))
            .bg_alpha(1.) // See comment on fn redraw_skia
            .flags(
                #[rustfmt::skip]
                        imgui::WindowFlags::NO_RESIZE
                    | imgui::WindowFlags::NO_COLLAPSE,
            )
            .position_pivot([0.5, 0.5])
            .position(
                [(v.viewport.winsize.0/2) as f32, (v.viewport.winsize.1/2) as f32],
                imgui::Condition::Always,
            )
            .size([TOOLBOX_HEIGHT, TOOLBOX_WIDTH+10.], imgui::Condition::Always)
            .focused(true)
            .build(ui, || {
                PROMPT_STR.with(|prompt_str| {
                    ui.push_item_width(-1.);
                    prompt_str.borrow_mut().clear();
                    ui.input_text(imgui::im_str!(""), &mut prompt_str.borrow_mut())
                    .build();
    
                    if ui.is_key_down(Key::Enter) {
                        let final_string = prompt_str.borrow().to_string();
                        let mut new_string = imgui::ImString::new("");
                        new_string.reserve(256);
                        prompt_str.replace(new_string);
                        func(v, final_string);
                        v.prompts.pop();
                    }
                })
            });
        }

        InputPrompt::Color { label, default, func } => {
            let mut color = PROMPT_CLR.with(|clr| clr.borrow_mut().clone() );

            imgui::Window::new(&imgui::im_str!("{}", label))
            .bg_alpha(1.) // See comment on fn redraw_skia
            .flags(
                #[rustfmt::skip]
                        imgui::WindowFlags::NO_RESIZE
                    | imgui::WindowFlags::NO_COLLAPSE,
            )
            .position_pivot([0.5, 0.5])
            .position(
                [(v.viewport.winsize.0/2) as f32, (v.viewport.winsize.1/2) as f32],
                imgui::Condition::Always,
            )
            .size([TOOLBOX_HEIGHT, TOOLBOX_HEIGHT + 10.], imgui::Condition::Always)
            .focused(true)
            .build(ui, || {
                PROMPT_CLR.with(|ui_color| {
                    imgui::ColorPicker::new(&imgui::im_str!("{}", label), &mut color)
                    .build(ui);        
    
                    if ui.is_key_down(Key::Enter) {
                        ui_color.replace([0., 0., 0., 1.]);
                        func(v, color);
                        v.prompts.pop();
                    }
                })
            });

            PROMPT_CLR.with(|clr| clr.replace(color));
        }
    }


}

thread_local! { pub static PROMPT_STR: RefCell<imgui::ImString> = RefCell::new(imgui::ImString::new("")); }
thread_local! { pub static PROMPT_CLR: RefCell<[f32; 4]> = RefCell::new([0., 0., 0., 1.]); }
thread_local! { pub static FONT_IDS: RefCell<Vec<FontId>> = RefCell::new(vec!()); }
