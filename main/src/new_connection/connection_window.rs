use gpui::{
    AnyView, AnyWindowHandle, App, Context, Entity, FocusHandle, Focusable, KeyBinding, Window,
    actions,
};

use crate::home_tab::HomePage;
use crate::new_connection::connection_kind::{NewConnectionCategory, NewConnectionKind};
use crate::new_connection::form_page::{NewConnectionFormPage, NewConnectionFormResult};

pub(super) const KEY_CONTEXT: &str = "NewConnectionWindow";

actions!(
    new_connection_window,
    [
        SelectPreviousConnectionKind,
        SelectNextConnectionKind,
        OpenConnectionKind
    ]
);

pub(crate) struct NewConnectionWindow {
    parent: Entity<HomePage>,
    parent_window: AnyWindowHandle,
    pub(super) focus_handle: FocusHandle,
    pub(super) selected_category: NewConnectionCategory,
    pub(super) selected_kind: Option<NewConnectionKind>,
    pub(super) form: Option<AnyView>,
}

impl NewConnectionWindow {
    pub(crate) fn new(
        parent: Entity<HomePage>,
        parent_window: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.bind_keys([
            KeyBinding::new("up", SelectPreviousConnectionKind, Some(KEY_CONTEXT)),
            KeyBinding::new("left", SelectPreviousConnectionKind, Some(KEY_CONTEXT)),
            KeyBinding::new("down", SelectNextConnectionKind, Some(KEY_CONTEXT)),
            KeyBinding::new("right", SelectNextConnectionKind, Some(KEY_CONTEXT)),
            KeyBinding::new("enter", OpenConnectionKind, Some(KEY_CONTEXT)),
        ]);

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        Self {
            parent,
            parent_window,
            focus_handle,
            selected_category: NewConnectionCategory::All,
            selected_kind: Self::first_visible_item(NewConnectionCategory::All),
            form: None,
        }
    }

    pub(super) fn first_visible_item(category: NewConnectionCategory) -> Option<NewConnectionKind> {
        NewConnectionKind::all()
            .into_iter()
            .find(|kind| category == NewConnectionCategory::All || kind.category() == category)
    }

    pub(super) fn visible_items(&self) -> Vec<NewConnectionKind> {
        NewConnectionKind::all()
            .into_iter()
            .filter(|kind| {
                self.selected_category == NewConnectionCategory::All
                    || kind.category() == self.selected_category
            })
            .collect()
    }

    fn select_visible_item(&mut self, offset: isize, cx: &mut Context<Self>) {
        let items = self.visible_items();
        if items.is_empty() {
            return;
        }

        let current_index = self
            .selected_kind
            .as_ref()
            .and_then(|selected| items.iter().position(|kind| kind == selected));
        let next_index = match current_index {
            Some(index) => (index as isize + offset).rem_euclid(items.len() as isize) as usize,
            None if offset < 0 => items.len() - 1,
            None => 0,
        };

        self.selected_kind = Some(items[next_index].clone());
        cx.notify();
    }

    pub(super) fn open_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.selected_kind.clone() else {
            return;
        };

        match kind.build_form_view(self.parent.clone(), self.parent_window, window, cx) {
            NewConnectionFormResult::Form(form) => {
                self.form = Some(form);
                cx.notify();
            }
            NewConnectionFormResult::Done => {
                window.remove_window();
            }
            NewConnectionFormResult::Blocked => {
                cx.notify();
            }
        }
    }

    pub(super) fn go_back_to_selection(&mut self, cx: &mut Context<Self>) {
        self.form = None;
        cx.notify();
    }

    pub(super) fn on_action_select_previous(
        &mut self,
        _: &SelectPreviousConnectionKind,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_visible_item(-1, cx);
    }

    pub(super) fn on_action_select_next(
        &mut self,
        _: &SelectNextConnectionKind,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_visible_item(1, cx);
    }

    pub(super) fn on_action_open_selected(
        &mut self,
        _: &OpenConnectionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_selected(window, cx);
    }
}

impl Focusable for NewConnectionWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
