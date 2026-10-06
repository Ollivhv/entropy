use super::*;

impl EntropyApp {
    /// Reads every Qube screen setting the firmware advertises.
    ///
    /// Unsupported QSIDs are skipped, so the page only ever shows rows the
    /// device can actually store. Values are read while connecting, exactly
    /// like the macropad display page, so the page is populated the moment it
    /// is opened.
    pub(super) fn read_qube_screen_settings(
        supported_qmk_settings: &[u16],
        dev_conn: &crate::hid::HidDevice,
    ) -> QubeScreenSettingsState {
        let mut state = QubeScreenSettingsState {
            fields: qube_screen_fields(supported_qmk_settings),
            ..QubeScreenSettingsState::default()
        };
        state.supported = !state.fields.is_empty();

        for field in state.fields.clone() {
            match field.kind {
                QubeScreenFieldKind::Text { .. } => {
                    match dev_conn.get_qmk_setting_string(field.qsid) {
                        Ok(value) => {
                            state.strings.insert(field.qsid, value.clone());
                            state.confirmed_strings.insert(field.qsid, value);
                        }
                        Err(error) => log::warn!(
                            "get_qmk_setting_string(qube screen qsid {}): {error}",
                            field.qsid
                        ),
                    }
                }
                QubeScreenFieldKind::Color { qsids } => {
                    for qsid in qsids {
                        match dev_conn.get_qmk_setting_u8(qsid) {
                            Ok(value) => {
                                state.confirmed.insert(qsid, u16::from(value));
                                state.values.insert(qsid, u16::from(value));
                            }
                            Err(error) => {
                                log::warn!("get_qmk_setting_u8(qube screen qsid {qsid}): {error}")
                            }
                        }
                    }
                }
                _ => match dev_conn.get_qmk_setting_u8(field.qsid) {
                    Ok(value) => {
                        state.confirmed.insert(field.qsid, u16::from(value));
                        state.values.insert(field.qsid, u16::from(value));
                    }
                    Err(error) => log::warn!(
                        "get_qmk_setting_u8(qube screen qsid {}): {error}",
                        field.qsid
                    ),
                },
            }
        }

        state
    }

    /// True when the firmware exposes the Qube screen block, which is what
    /// drives the "Qube screen" menu row.
    pub(super) fn qube_screen_settings_available(&self) -> bool {
        self.qube_screen_settings.supported
    }

    pub(super) fn draw_qube_screen_settings_page(
        &mut self,
        ui: &mut egui::Ui,
        content_rect: egui::Rect,
    ) {
        let lang = self.app_settings.language;
        let dark = ui.visuals().dark_mode;
        let hid_ready = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.qmk_setting_transport_available()
            }
            #[cfg(target_arch = "wasm32")]
            {
                false
            }
        };

        crate::ui_style::allocate_ui_at_rect(ui, content_rect, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                ui.label(
                    RichText::new(crate::i18n::tr_catalog(lang, "qube_screen.title"))
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(crate::i18n::tr_catalog(lang, "qube_screen.description"))
                        .size(13.0)
                        .color(app_muted_text(dark)),
                );
                ui.add_space(24.0);

                if !self.qube_screen_settings.supported {
                    crate::ui_style::modal_empty_state(
                        ui,
                        crate::i18n::tr_catalog(lang, "qube_screen.unavailable"),
                        Some(crate::i18n::tr(
                            lang,
                            crate::i18n::Key::QmkSettingsEnableHint,
                        )),
                    );
                    return;
                }

                if !hid_ready {
                    crate::ui_style::modal_empty_state(
                        ui,
                        crate::i18n::tr_catalog(lang, "qube_screen.connect"),
                        None,
                    );
                    return;
                }

                let metrics = crate::ui_style::ResponsiveMetrics::from_ctx(ui.ctx());
                let row_count = self.qube_screen_settings.fields.len();
                let list = allocate_adaptive_settings_list_viewport(
                    ui,
                    "qube_screen_settings",
                    metrics,
                    row_count,
                    0.0,
                );
                crate::ui_style::allocate_ui_at_rect(ui, list.content_rect, |ui| {
                    ui.set_clip_rect(list.viewport);
                    ui.set_min_size(list.content_rect.size());
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.draw_qube_screen_rows(
                        ui,
                        list.first_visible_row..list.last_visible_row,
                        metrics,
                        list.suppress_tooltips,
                    );
                });

                if list.has_scrollbar {
                    crate::ui_style::paint_floating_scrollbar_handle(
                        ui,
                        list.track_rect,
                        list.handle_height,
                        list.scroll_ratio,
                        list.track_hovered,
                    );
                }
            });
        });
    }

    fn draw_qube_screen_rows(
        &mut self,
        ui: &mut egui::Ui,
        row_range: std::ops::Range<usize>,
        metrics: crate::ui_style::ResponsiveMetrics,
        suppress_tooltips: bool,
    ) {
        let dark = ui.visuals().dark_mode;
        let content_width = metrics.settings_row_content_width();
        let row_height = metrics.settings_row_height();
        let control_height = metrics.settings_control_height();
        let control_font = metrics.settings_control_font_size();
        let scale = metrics.scale;
        let dropdown_width = metrics.value(176.0);
        let switch_width = metrics.value(46.0);
        let switch_size = metrics.size(46.0, 24.0);
        let swatch_width = metrics.value(70.0);
        let text_width = metrics.value(120.0);

        for row_idx in row_range {
            let Some(field) = self.qube_screen_settings.fields.get(row_idx).copied() else {
                continue;
            };
            let label = crate::i18n::tr_catalog(self.app_settings.language, field.label_key);
            let tooltip = (!suppress_tooltips)
                .then(|| crate::i18n::tr_catalog(self.app_settings.language, field.tooltip_key));

            match field.kind {
                QubeScreenFieldKind::Toggle => {
                    let mut enabled = self.qube_screen_settings.value(field.qsid) != 0;
                    let mut write = None;
                    crate::ui_style::settings_list_row_with_tooltip(
                        ui,
                        content_width,
                        row_height,
                        label,
                        true,
                        tooltip,
                        switch_width,
                        |ui| {
                            let response = crate::ui_style::settings_switch_sized_stable(
                                ui,
                                ("qube_screen_toggle", field.qsid),
                                &mut enabled,
                                switch_size,
                            );
                            if response.changed() {
                                write = Some(u16::from(enabled));
                            }
                        },
                    );
                    if let Some(value) = write {
                        self.apply_qube_screen_value(field.qsid, value, label);
                    }
                }
                QubeScreenFieldKind::Select { variants } => {
                    let labels: Vec<String> = variants
                        .iter()
                        .map(|key| {
                            crate::i18n::tr_catalog(self.app_settings.language, key).to_owned()
                        })
                        .collect();
                    let selected = (self.qube_screen_settings.value(field.qsid) as usize)
                        .min(labels.len().saturating_sub(1));
                    let mut picked = None;
                    crate::ui_style::settings_list_row_with_tooltip(
                        ui,
                        content_width,
                        row_height,
                        label,
                        true,
                        tooltip,
                        dropdown_width,
                        |ui| {
                            let dropdown_id =
                                ui.make_persistent_id(("qube_screen_dropdown", field.qsid));
                            let (_, choice) = crate::ui_style::modern_dropdown_select_sized(
                                ui,
                                dropdown_id,
                                &labels,
                                selected,
                                dropdown_width,
                                control_height,
                                control_font,
                            );
                            picked = choice;
                        },
                    );
                    if let Some(index) = picked {
                        self.apply_qube_screen_value(field.qsid, index as u16, label);
                    }
                }
                QubeScreenFieldKind::Number { min, max } => {
                    let mut value = self.qube_screen_settings.value(field.qsid) as f32;
                    let suffix = qube_screen_number_suffix(self.app_settings.language, field.qsid);
                    let mut write = None;
                    crate::ui_style::settings_list_row_with_tooltip(
                        ui,
                        content_width,
                        row_height,
                        label,
                        true,
                        tooltip,
                        metrics.value(196.0),
                        |ui| {
                            ui.visuals_mut().selection.bg_fill = app_accent();
                            ui.visuals_mut().widgets.active.bg_fill = app_accent();
                            ui.visuals_mut().widgets.active.weak_bg_fill = app_accent();
                            ui.visuals_mut().widgets.hovered.bg_stroke =
                                Stroke::new(1.0_f32, app_accent());
                            let value_color = if ui.visuals().dark_mode {
                                Color32::from_gray(230)
                            } else {
                                Color32::from_gray(55)
                            };
                            let mut changed = false;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_sized(
                                        [48.0 * scale, 34.0 * scale],
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "{}{suffix}",
                                                value.round() as u8
                                            ))
                                            .size(12.0 * scale)
                                            .color(value_color),
                                        )
                                        .halign(egui::Align::RIGHT),
                                    );
                                    ui.spacing_mut().slider_width = 132.0 * scale;
                                    changed = ui
                                        .add_sized(
                                            [132.0 * scale, 34.0 * scale],
                                            egui::Slider::new(
                                                &mut value,
                                                f32::from(min)..=f32::from(max),
                                            )
                                            .step_by(1.0)
                                            .show_value(false)
                                            .trailing_fill(true),
                                        )
                                        .changed();
                                },
                            );
                            if changed {
                                write =
                                    Some(value.round().clamp(f32::from(min), f32::from(max)) as u16);
                            }
                        },
                    );
                    if let Some(value) = write {
                        self.apply_qube_screen_value(field.qsid, value, label);
                    }
                }
                QubeScreenFieldKind::Color { qsids } => {
                    let rgb = [
                        self.qube_screen_settings.value(qsids[0]).min(255) as u8,
                        self.qube_screen_settings.value(qsids[1]).min(255) as u8,
                        self.qube_screen_settings.value(qsids[2]).min(255) as u8,
                    ];
                    let mut write = None;
                    crate::ui_style::settings_list_row_with_tooltip(
                        ui,
                        content_width,
                        row_height,
                        label,
                        true,
                        tooltip,
                        swatch_width,
                        |ui| {
                            write = self.draw_qube_screen_color_swatch(ui, dark, scale, qsids, rgb);
                        },
                    );
                    if let Some(color) = write {
                        for (index, qsid) in qsids.iter().copied().enumerate() {
                            self.apply_qube_screen_value(qsid, u16::from(color[index]), label);
                        }
                    }
                }
                QubeScreenFieldKind::Text { max_chars } => {
                    let mut text = self.qube_screen_settings.string(field.qsid).to_owned();
                    crate::ui_style::settings_list_row_with_tooltip(
                        ui,
                        content_width,
                        row_height,
                        label,
                        true,
                        tooltip,
                        text_width,
                        |ui| {
                            let response = crate::ui_style::modern_text_field_sized(
                                ui,
                                ui.make_persistent_id(("qube_screen_text", field.qsid)),
                                &mut text,
                                text_width,
                                control_height,
                                "",
                                max_chars,
                                egui::Align::Min,
                            );
                            if response.changed() {
                                self.qube_screen_settings
                                    .strings
                                    .insert(field.qsid, text.clone());
                            }
                            let submitted = response.lost_focus()
                                || (response.has_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                            if submitted {
                                self.write_qube_screen_string(field.qsid, label);
                            }
                        },
                    );
                }
            }
        }
    }

    fn draw_qube_screen_color_swatch(
        &mut self,
        ui: &mut egui::Ui,
        dark: bool,
        scale: f32,
        qsids: [u16; 3],
        rgb: [u8; 3],
    ) -> Option<[u8; 3]> {
        let popup_id = ui.make_persistent_id(("qube_screen_color_popup", qsids[0]));
        let hsva_id = popup_id.with("hsva");
        let popup_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let border = if dark {
            Color32::from_gray(95)
        } else {
            Color32::from_gray(185)
        };
        let swatch_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let (swatch_rect, swatch_response) =
            ui.allocate_exact_size(Vec2::new(64.0 * scale, 34.0 * scale), Sense::click());
        if swatch_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if swatch_response.clicked() {
            ui.ctx().data_mut(|data| {
                data.insert_temp(hsva_id, egui::ecolor::Hsva::from(swatch_color));
            });
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }
        ui.painter().rect(
            swatch_rect,
            9.0,
            app_surface_fill(dark),
            Stroke::new(
                1.0_f32,
                if popup_open { app_accent() } else { border },
            ),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect(
            swatch_rect.shrink(5.0 * scale),
            6.0,
            swatch_color,
            Stroke::new(1.0_f32, border.gamma_multiply(0.85)),
            egui::StrokeKind::Inside,
        );

        let mut picked_hsva = ui
            .ctx()
            .data(|data| data.get_temp::<egui::ecolor::Hsva>(hsva_id))
            .unwrap_or_else(|| swatch_color.into());
        let mut write = None;
        crate::ui_style::popup_below_widget(
            ui,
            popup_id,
            &swatch_response,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.spacing_mut().slider_width = 136.0 * scale;
                if super::rgb_settings_ui::compact_rgb_color_picker(ui, &mut picked_hsva) {
                    let color: Color32 = picked_hsva.into();
                    write = Some([color.r(), color.g(), color.b()]);
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(hsva_id, picked_hsva));
                }
            },
        );
        write
    }

    /// Applies a numeric Qube screen edit: shows it immediately, then queues
    /// the firmware write through the shared settings write queue, so the
    /// device readback stays the source of truth.
    fn apply_qube_screen_value(&mut self, qsid: u16, value: u16, label: &str) {
        if value == self.qube_screen_settings.value(qsid) {
            return;
        }
        self.qube_screen_settings.set_value(qsid, value);
        let confirmed = self.qube_screen_settings.confirmed_value(qsid);
        let already_requested = self.pending_settings_write_value(qsid).unwrap_or(confirmed)
            == value;
        if !already_requested {
            self.queue_qube_screen_setting_write(label.to_owned(), qsid, confirmed, value);
        }
    }

    /// Writes a battery label. Strings cannot travel through the numeric write
    /// queue, so this mirrors the layer-name path: a direct firmware write with
    /// the confirmed copy kept for the "unchanged?" check.
    fn write_qube_screen_string(&mut self, qsid: u16, label: &str) {
        let value = self.qube_screen_settings.string(qsid).to_owned();
        let confirmed = self
            .qube_screen_settings
            .confirmed_strings
            .get(&qsid)
            .cloned()
            .unwrap_or_default();
        if value == confirmed {
            return;
        }
        let Some(hid) = &self.hid_device else {
            return;
        };
        match hid.set_qmk_setting_string(qsid, &value) {
            Ok(()) => {
                self.qube_screen_settings
                    .confirmed_strings
                    .insert(qsid, value);
            }
            Err(error) => {
                let error_text = error.to_string();
                self.status_msg = crate::i18n::tr_catalog_format(
                    self.app_settings.language,
                    "settings_write.failed_status",
                    &[("setting", label), ("error", error_text.as_str())],
                );
                log::warn!("set_qmk_setting_string(qube screen qsid {qsid}) failed: {error_text}");
            }
        }
    }
}

/// Value suffix of the numeric Qube screen rows.
fn qube_screen_number_suffix(language: crate::i18n::Language, qsid: u16) -> &'static str {
    match qsid {
        218 => match language {
            crate::i18n::Language::Russian => " с",
            crate::i18n::Language::English => " s",
        },
        318 => "%",
        _ => "",
    }
}
