use crate::home::home_workspace_filter::{WorkspaceFilterDelegate, show_workspace_dialog};
use crate::home_tab::HomePage;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Styled as _,
    Window, div, px,
};
use gpui_component::{
    IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    chrome, h_flex,
    list::{List, ListState},
    popover::Popover,
};
use rust_i18n::t;

#[derive(Clone)]
struct WorkspaceFilterContent {
    view_for_select: Entity<HomePage>,
    view_for_clear: Entity<HomePage>,
    view_for_new: Entity<HomePage>,
    list: Entity<ListState<WorkspaceFilterDelegate>>,
    is_all_selected: bool,
}

impl HomePage {
    pub(crate) fn render_workspace_filter_popover(
        &mut self,
        open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let list = self.ensure_workspace_filter_list(window, cx);
        self.refresh_workspace_filter_list(&list, cx);

        let content = WorkspaceFilterContent {
            view_for_select: view.clone(),
            view_for_clear: view.clone(),
            view_for_new: view,
            list,
            is_all_selected: self.all_workspaces_selected(),
        };

        Popover::new("workspace-filter-popover")
            .trigger(
                Button::new("workspace-filter")
                    .icon(IconName::Filter)
                    .tooltip(t!("Workspace.filter")),
            )
            .open(open)
            .on_open_change(cx.listener(|this, open, _, cx| {
                this.workspace_filter_open = *open;
                cx.notify();
            }))
            .content(move |_, _, cx| workspace_filter_content(content.clone(), cx))
            .into_any_element()
    }

    pub(crate) fn ensure_workspace_filter_list(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ListState<WorkspaceFilterDelegate>> {
        if let Some(ref list) = self.workspace_filter_list {
            return list.clone();
        }

        let parent = cx.entity();
        let list = cx.new(|cx| {
            ListState::new(WorkspaceFilterDelegate::new(parent), window, cx).searchable(true)
        });
        self.workspace_filter_list = Some(list.clone());
        list
    }

    fn refresh_workspace_filter_list(
        &self,
        list: &Entity<ListState<WorkspaceFilterDelegate>>,
        cx: &mut Context<Self>,
    ) {
        list.update(cx, |state, _cx| {
            state.delegate_mut().update_items_with_data(
                &self.workspaces,
                &self.connections,
                &self.filtered_workspace_ids,
            );
        });
    }

    fn all_workspaces_selected(&self) -> bool {
        self.filtered_workspace_ids.is_empty()
            || self.filtered_workspace_ids.len()
                == self.workspaces.iter().filter(|w| w.id.is_some()).count()
    }
}

fn workspace_filter_content(content: WorkspaceFilterContent, cx: &mut gpui::App) -> AnyElement {
    chrome::panel_surface(cx)
        .w(px(280.0))
        .max_h(px(400.0))
        .child(workspace_filter_header(&content, cx))
        .child(
            List::new(&content.list)
                .w_full()
                .max_h(px(320.0))
                .p(px(8.))
                .flex_1(),
        )
        .into_any_element()
}

fn workspace_filter_header(content: &WorkspaceFilterContent, cx: &mut gpui::App) -> AnyElement {
    chrome::panel_header(cx)
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(workspace_filter_select_all(content))
                .child(workspace_filter_actions(content)),
        )
        .into_any_element()
}

fn workspace_filter_select_all(content: &WorkspaceFilterContent) -> AnyElement {
    let view_select = content.view_for_select.clone();
    h_flex()
        .gap_2()
        .items_center()
        .child(
            Checkbox::new("select-all-ws")
                .checked(content.is_all_selected)
                .on_click(move |_, _, cx| {
                    view_select.update(cx, |this, cx| {
                        if this.all_workspaces_selected() {
                            this.clear_workspace_filter(cx);
                        } else {
                            this.select_all_workspaces(cx);
                        }
                    });
                }),
        )
        .child(
            div()
                .text_sm()
                .child(t!("Workspace.select_all").to_string().into_any_element()),
        )
        .into_any_element()
}

fn workspace_filter_actions(content: &WorkspaceFilterContent) -> AnyElement {
    h_flex()
        .gap_1()
        .child(new_workspace_button(content.view_for_new.clone()))
        .child(clear_workspace_filter_button(
            content.view_for_clear.clone(),
        ))
        .into_any_element()
}

fn new_workspace_button(view_new: Entity<HomePage>) -> impl IntoElement {
    Button::new("new-workspace-from-filter")
        .primary()
        .small()
        .label(t!("Common.new"))
        .on_click(move |_, window, cx| {
            show_workspace_dialog(view_new.clone(), None, String::new(), window, cx);
        })
}

fn clear_workspace_filter_button(view_clear: Entity<HomePage>) -> impl IntoElement {
    Button::new("clear-ws-filter")
        .ghost()
        .small()
        .label(t!("Workspace.clear_filter"))
        .on_click(move |_, _, cx| {
            view_clear.update(cx, |this, cx| {
                this.clear_workspace_filter(cx);
            });
        })
}
