#[allow(dead_code)]
#[derive(PartialEq,Debug,Clone,Copy)]
pub enum ButtonKeyState{
    Pressed,
    Released,
}
#[allow(dead_code)]
#[derive(PartialEq,Debug,Clone,Copy)]
pub enum EnterLeave{
    Enter,
    Leave,
}
#[allow(dead_code)]
#[derive(PartialEq,Debug,Clone,Copy)]
pub enum FocusChange{
    Gained,
    Lost,
}
#[allow(dead_code)]
#[derive(PartialEq,Debug,Clone,Copy)]
pub enum Event{
    FirstEvent,
    LastEvent,
    ButtonEvent{
        button:u32,
        button_state:ButtonKeyState,
        x:u16,
        y:u16,
        enter_leave:Option<EnterLeave>,

    },
    KeyEvent{
        key:u32,
        key_state:ButtonKeyState,
        x:u16,
        y:u16,
    },
    FocusEvent{
        focus_change:FocusChange,
    },
    ResizeRequestEvent,
    WindowCloseRequested,
    #[allow(dead_code)]
    None,
}