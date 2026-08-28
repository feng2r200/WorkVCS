mod session;

pub use session::{
    SessionEndOptions, SessionEndResult, SessionFocus, SessionFocusOptions, SessionFocusPathEntry,
    SessionFocusUpdateResult, SessionLifecycleState, SessionSnapshot, SessionStartOptions,
    SessionStartResult,
};

pub(crate) use session::{
    clear_session_focus, end_session, session_snapshot, set_session_focus, start_session,
};
