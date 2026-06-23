use std::collections::HashSet;

use chrono::{DateTime, Local, TimeZone, Utc};
use gpui::{
    App, AppContext, AsyncApp, Context, FocusHandle, Focusable, FontWeight, IntoElement,
    ParentElement, Render, Styled, WeakEntity, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    pagination::Pagination,
    scroll::ScrollableElement,
    v_flex,
};
use rust_i18n::t;

use crate::setting_tab::AppSettings;
use crate::webdav_backup::{self, RemoteBackupEntry};
use one_core::popup_window::{PopupWindowOptions, open_popup_window};
use one_core::storage::manager::get_config_dir;

const PAGE_SIZE: usize = 6;
const DIALOG_TOP_INSET: f32 = 48.0;

pub(crate) fn show_webdav_backup_manager_window(cx: &mut App) {
    open_popup_window(
        PopupWindowOptions::new(t!("Settings.Backup.WebDav.manage_title").to_string())
            .size(840.0, 520.0),
        move |window, cx| cx.new(|cx| WebDavBackupManagerView::new(window, cx)),
        cx,
    );
}

struct WebDavBackupManagerView {
    focus_handle: FocusHandle,
    backups: Vec<RemoteBackupEntry>,
    selected: HashSet<String>,
    page: usize,
    loading: bool,
    acting: bool,
    status_message: Option<(bool, String)>,
}

impl WebDavBackupManagerView {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            focus_handle: cx.focus_handle(),
            backups: Vec::new(),
            selected: HashSet::new(),
            page: 1,
            loading: false,
            acting: false,
            status_message: None,
        };
        view.load_backups(cx);
        view
    }

    fn load_backups(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        self.loading = true;
        self.status_message = None;
        cx.notify();

        let settings = AppSettings::global(cx).webdav_backup.clone();
        let http_client = cx.http_client();

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = webdav_backup::list_backups(settings, http_client).await;
            let _ = this.update(cx, |view, cx| {
                view.loading = false;
                match result {
                    Ok(backups) => {
                        view.backups = backups;
                        view.selected
                            .retain(|name| view.backups.iter().any(|item| &item.file_name == name));
                        let total_pages = view.total_pages();
                        if view.page > total_pages {
                            view.page = total_pages;
                        }
                    }
                    Err(err) => {
                        view.status_message = Some((false, err));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn stage_restore(&mut self, file_name: String, cx: &mut Context<Self>) {
        if self.acting {
            return;
        }

        let config_dir = match get_config_dir() {
            Ok(dir) => dir,
            Err(err) => {
                self.status_message = Some((false, err.to_string()));
                cx.notify();
                return;
            }
        };

        self.acting = true;
        self.status_message = None;
        cx.notify();

        let settings = AppSettings::global(cx).webdav_backup.clone();
        let http_client = cx.http_client();

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = webdav_backup::stage_restore_from_webdav(
                settings,
                http_client,
                config_dir,
                &file_name,
            )
            .await;

            let _ = this.update(cx, |view, cx| {
                view.acting = false;
                view.status_message = Some(match result {
                    Ok(_) => (
                        true,
                        t!("Settings.Backup.WebDav.restore_staged").to_string(),
                    ),
                    Err(err) => (false, err),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn delete_backups(&mut self, file_names: Vec<String>, cx: &mut Context<Self>) {
        if self.acting || file_names.is_empty() {
            return;
        }

        self.acting = true;
        self.status_message = None;
        cx.notify();

        let settings = AppSettings::global(cx).webdav_backup.clone();
        let http_client = cx.http_client();

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut result = Ok(0usize);
            let mut deleted_count = 0usize;
            for file_name in &file_names {
                match webdav_backup::delete_backup(settings.clone(), http_client.clone(), file_name)
                    .await
                {
                    Ok(()) => deleted_count += 1,
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                }
            }
            let _ = this.update(cx, |view, cx| {
                view.acting = false;
                match result {
                    Ok(_) => {
                        view.status_message = Some((
                            true,
                            t!(
                                "Settings.Backup.WebDav.delete_success",
                                count = deleted_count
                            )
                            .to_string(),
                        ));
                        for file_name in &file_names {
                            view.selected.remove(file_name);
                        }
                        view.load_backups(cx);
                    }
                    Err(err) => {
                        view.status_message = Some((false, err));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn total_pages(&self) -> usize {
        self.backups.len().div_ceil(PAGE_SIZE).max(1)
    }

    fn page_slice(&self) -> &[RemoteBackupEntry] {
        let start = self.page.saturating_sub(1) * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(self.backups.len());
        if start >= end {
            &[]
        } else {
            &self.backups[start..end]
        }
    }

    fn all_page_selected(&self) -> bool {
        let page_items = self.page_slice();
        !page_items.is_empty()
            && page_items
                .iter()
                .all(|item| self.selected.contains(&item.file_name))
    }

    fn selected_count(&self) -> usize {
        self.selected.len()
    }

    fn render_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let rows: Vec<_> = self
            .page_slice()
            .iter()
            .map(|item| {
                self.render_backup_row(view.clone(), item, cx)
                    .into_any_element()
            })
            .collect();

        if self.loading {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(t!("Settings.Backup.WebDav.loading").to_string())
                .into_any_element();
        }

        if self.backups.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(t!("Settings.Backup.WebDav.empty").to_string())
                .into_any_element();
        }

        v_flex()
            .flex_1()
            .min_h_0()
            .child(self.render_table_header(view.clone(), cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(v_flex().children(rows)),
            )
            .into_any_element()
    }

    fn render_table_header(
        &self,
        view: gpui::Entity<Self>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let checked = self.all_page_selected();
        let checkbox_view = view.clone();
        h_flex()
            .w_full()
            .h(px(48.0))
            .items_center()
            .px_3()
            .gap_3()
            .bg(cx.theme().table_head)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div().w(px(28.0)).child(
                    Checkbox::new("backup-select-page")
                        .checked(checked)
                        .disabled(self.acting)
                        .on_click(move |next, _window, cx| {
                            let _ = checkbox_view.update(cx, |this, cx| {
                                let names: Vec<String> = this
                                    .page_slice()
                                    .iter()
                                    .map(|item| item.file_name.clone())
                                    .collect();
                                if *next {
                                    for name in names {
                                        this.selected.insert(name);
                                    }
                                } else {
                                    for name in names {
                                        this.selected.remove(&name);
                                    }
                                }
                                cx.notify();
                            });
                        }),
                ),
            )
            .child(header_cell(
                t!("Settings.Backup.WebDav.file_name").to_string(),
                1.0,
                None,
                cx,
            ))
            .child(header_cell(
                t!("Settings.Backup.WebDav.modified_at").to_string(),
                0.0,
                Some(180.0),
                cx,
            ))
            .child(header_cell(
                t!("Settings.Backup.WebDav.size").to_string(),
                0.0,
                Some(110.0),
                cx,
            ))
            .child(header_cell(
                t!("Settings.Backup.WebDav.actions").to_string(),
                0.0,
                Some(150.0),
                cx,
            ))
    }

    fn render_backup_row(
        &self,
        view: gpui::Entity<Self>,
        item: &RemoteBackupEntry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected.contains(&item.file_name);
        let file_name = item.file_name.clone();
        let restore_file = item.file_name.clone();
        let delete_file = item.file_name.clone();
        let checkbox_view = view.clone();
        let restore_view = view.clone();
        let delete_view = view.clone();

        h_flex()
            .w_full()
            .h(px(56.0))
            .items_center()
            .px_3()
            .gap_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div().w(px(28.0)).child(
                    Checkbox::new(format!("backup-check-{}", file_name))
                        .checked(selected)
                        .disabled(self.acting)
                        .on_click(move |next, _window, cx| {
                            let _ = checkbox_view.update(cx, |this, cx| {
                                if *next {
                                    this.selected.insert(file_name.clone());
                                } else {
                                    this.selected.remove(&file_name);
                                }
                                cx.notify();
                            });
                        }),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .truncate()
                    .child(item.file_name.clone()),
            )
            .child(fixed_cell(
                format_timestamp(item.modified_at),
                180.0,
                cx.theme().foreground,
            ))
            .child(fixed_cell(
                format_bytes(item.size_bytes),
                110.0,
                cx.theme().muted_foreground,
            ))
            .child(
                h_flex()
                    .w(px(150.0))
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new(format!("restore-{}", item.file_name))
                            .small()
                            .ghost()
                            .label(t!("Home.restore").to_string())
                            .disabled(self.acting)
                            .text_color(cx.theme().primary)
                            .on_click({
                                move |_, _, cx| {
                                    let _ = restore_view.update(cx, |this, cx| {
                                        this.stage_restore(restore_file.clone(), cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new(format!("delete-{}", item.file_name))
                            .small()
                            .ghost()
                            .label(t!("Common.delete").to_string())
                            .disabled(self.acting)
                            .text_color(cx.theme().danger)
                            .on_click(move |_, _, cx| {
                                let _ = delete_view.update(cx, |this, cx| {
                                    this.delete_backups(vec![delete_file.clone()], cx);
                                });
                            }),
                    ),
            )
    }
}

impl Focusable for WebDavBackupManagerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WebDavBackupManagerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let total_pages = self.total_pages();
        let selected_count = self.selected_count();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .pt(px(DIALOG_TOP_INSET))
            .child(
                v_flex()
                    .gap_1()
                    .px_5()
                    .pb_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t!("Settings.Backup.WebDav.manage_title").to_string()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("Settings.Backup.WebDav.manage_desc").to_string()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_5()
                    .pt_4()
                    .child(self.render_table(cx)),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_5()
                    .pb_4()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(
                                if self.status_message.as_ref().is_some_and(|(ok, _)| *ok) {
                                    cx.theme().muted_foreground
                                } else {
                                    cx.theme().danger
                                },
                            )
                            .child(
                                self.status_message
                                    .as_ref()
                                    .map(|(_, msg)| msg.clone())
                                    .unwrap_or_default(),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .when(total_pages > 1, |this| {
                                this.child(
                                    Pagination::new("backup-pagination")
                                        .current_page(self.page)
                                        .total_pages(total_pages)
                                        .on_click(move |page, _window, cx| {
                                            let _ = view.update(cx, |this, cx| {
                                                this.page = *page;
                                                cx.notify();
                                            });
                                        }),
                                )
                            })
                            .child(
                                Button::new("backup-refresh")
                                    .small()
                                    .ghost()
                                    .label(t!("Home.refresh").to_string())
                                    .disabled(self.loading || self.acting)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.load_backups(cx);
                                    })),
                            )
                            .child(
                                Button::new("backup-delete-selected")
                                    .small()
                                    .ghost()
                                    .label(
                                        t!(
                                            "Settings.Backup.WebDav.delete_selected",
                                            count = selected_count
                                        )
                                        .to_string(),
                                    )
                                    .disabled(self.acting || selected_count == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let names: Vec<String> =
                                            this.selected.iter().cloned().collect();
                                        this.delete_backups(names, cx);
                                    })),
                            )
                            .child(
                                Button::new("backup-close")
                                    .small()
                                    .label(t!("Common.close").to_string())
                                    .on_click(move |_, window, _| {
                                        window.remove_window();
                                    }),
                            ),
                    ),
            )
    }
}

fn header_cell(
    text: String,
    flex: f32,
    width: Option<f32>,
    cx: &mut Context<WebDavBackupManagerView>,
) -> impl IntoElement {
    let mut cell = div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().foreground);
    if let Some(width) = width {
        cell = cell.w(px(width));
    } else if flex > 0.0 {
        cell = cell.flex_1();
    }
    cell.child(text)
}

fn fixed_cell(text: String, width: f32, color: gpui::Hsla) -> impl IntoElement {
    div().w(px(width)).text_sm().text_color(color).child(text)
}

fn format_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| {
            DateTime::<Utc>::from_timestamp(timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        })
}

fn format_bytes(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{} B", size as u64)
    }
}
