use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::{align_of, size_of};

pub(crate) const ABI_VERSION: u32 = 1;
pub(crate) const CREATE_WITH_PARENT_ABI_VERSION: u32 = 1;
pub(crate) const SESSION_DISPLAY_SETTINGS_ABI_VERSION: u32 = 1;
pub(crate) const CONNECTION_FLAG_AUDIO_PLAYBACK_DISABLED: u32 = 1 << 0;
#[allow(dead_code)]
pub(crate) const CONNECTION_FLAGS_KNOWN: u32 = CONNECTION_FLAG_AUDIO_PLAYBACK_DISABLED;

pub(crate) const DISPLAY_FLAG_SMART_SIZING: u32 = 1 << 0;
pub(crate) const DISPLAY_FLAG_USE_MULTIMON: u32 = 1 << 1;
pub(crate) const DISPLAY_FLAG_SPAN_MONITORS: u32 = 1 << 2;
#[allow(dead_code)]
pub(crate) const DISPLAY_FLAGS_KNOWN: u32 =
    DISPLAY_FLAG_SMART_SIZING | DISPLAY_FLAG_USE_MULTIMON | DISPLAY_FLAG_SPAN_MONITORS;

pub(crate) const RESOURCE_FLAG_CLIPBOARD: u32 = 1 << 0;
pub(crate) const RESOURCE_FLAG_DRIVES: u32 = 1 << 1;
pub(crate) const RESOURCE_FLAG_DYNAMIC_DRIVES: u32 = 1 << 2;
pub(crate) const RESOURCE_FLAG_DYNAMIC_DEVICES: u32 = 1 << 3;
pub(crate) const RESOURCE_FLAG_PRINTERS: u32 = 1 << 4;
pub(crate) const RESOURCE_FLAG_SERIAL_PORTS: u32 = 1 << 5;
pub(crate) const RESOURCE_FLAG_SMART_CARDS: u32 = 1 << 6;
pub(crate) const RESOURCE_FLAG_CAMERAS: u32 = 1 << 7;
pub(crate) const RESOURCE_FLAG_MICROPHONES: u32 = 1 << 8;
pub(crate) const RESOURCE_FLAG_POS_DEVICES: u32 = 1 << 9;
#[allow(dead_code)]
pub(crate) const RESOURCE_FLAGS_KNOWN: u32 = RESOURCE_FLAG_CLIPBOARD
    | RESOURCE_FLAG_DRIVES
    | RESOURCE_FLAG_DYNAMIC_DRIVES
    | RESOURCE_FLAG_DYNAMIC_DEVICES
    | RESOURCE_FLAG_PRINTERS
    | RESOURCE_FLAG_SERIAL_PORTS
    | RESOURCE_FLAG_SMART_CARDS
    | RESOURCE_FLAG_CAMERAS
    | RESOURCE_FLAG_MICROPHONES
    | RESOURCE_FLAG_POS_DEVICES;

pub(crate) const AUDIO_FLAG_CAPTURE: u32 = 1 << 0;
#[allow(dead_code)]
pub(crate) const AUDIO_FLAGS_KNOWN: u32 = AUDIO_FLAG_CAPTURE;

pub(crate) const INPUT_FLAG_ENABLE_WINDOWS_KEY: u32 = 1 << 0;
pub(crate) const INPUT_FLAG_GRAB_FOCUS_ON_CONNECT: u32 = 1 << 1;
pub(crate) const INPUT_FLAGS_KNOWN: u32 =
    INPUT_FLAG_ENABLE_WINDOWS_KEY | INPUT_FLAG_GRAB_FOCUS_ON_CONNECT;

pub(crate) const PERFORMANCE_FLAG_WALLPAPER: u32 = 1 << 0;
pub(crate) const PERFORMANCE_FLAG_FULL_WINDOW_DRAG: u32 = 1 << 1;
pub(crate) const PERFORMANCE_FLAG_MENU_ANIMATIONS: u32 = 1 << 2;
pub(crate) const PERFORMANCE_FLAG_THEMES: u32 = 1 << 3;
pub(crate) const PERFORMANCE_FLAG_CURSOR_SHADOW: u32 = 1 << 4;
pub(crate) const PERFORMANCE_FLAG_CURSOR_SETTINGS: u32 = 1 << 5;
pub(crate) const PERFORMANCE_FLAG_FONT_SMOOTHING: u32 = 1 << 6;
pub(crate) const PERFORMANCE_FLAG_DESKTOP_COMPOSITION: u32 = 1 << 7;
pub(crate) const PERFORMANCE_FLAG_BITMAP_CACHE: u32 = 1 << 8;
pub(crate) const PERFORMANCE_FLAGS_KNOWN: u32 = PERFORMANCE_FLAG_WALLPAPER
    | PERFORMANCE_FLAG_FULL_WINDOW_DRAG
    | PERFORMANCE_FLAG_MENU_ANIMATIONS
    | PERFORMANCE_FLAG_THEMES
    | PERFORMANCE_FLAG_CURSOR_SHADOW
    | PERFORMANCE_FLAG_CURSOR_SETTINGS
    | PERFORMANCE_FLAG_FONT_SMOOTHING
    | PERFORMANCE_FLAG_DESKTOP_COMPOSITION
    | PERFORMANCE_FLAG_BITMAP_CACHE;

pub(crate) const SECURITY_FLAG_ENABLE_CREDSSP: u32 = 1 << 0;
pub(crate) const SECURITY_FLAG_PUBLIC_MODE: u32 = 1 << 1;
pub(crate) const SECURITY_FLAG_ENCRYPTION_ENABLED: u32 = 1 << 2;
#[allow(dead_code)]
pub(crate) const SECURITY_FLAGS_KNOWN: u32 =
    SECURITY_FLAG_ENABLE_CREDSSP | SECURITY_FLAG_PUBLIC_MODE | SECURITY_FLAG_ENCRYPTION_ENABLED;

pub(crate) const GATEWAY_FLAG_BYPASS_LOCAL: u32 = 1 << 0;
#[allow(dead_code)]
pub(crate) const GATEWAY_FLAGS_KNOWN: u32 = GATEWAY_FLAG_BYPASS_LOCAL;

pub(crate) const CONNECTION_FLAG_ADMIN_SESSION: u32 = 1 << 0;
pub(crate) const CONNECTION_FLAG_AUTO_RECONNECT: u32 = 1 << 1;
#[allow(dead_code)]
pub(crate) const CONNECTION_POLICY_FLAGS_KNOWN: u32 =
    CONNECTION_FLAG_ADMIN_SESSION | CONNECTION_FLAG_AUTO_RECONNECT;

pub(crate) type NativeResult = i32;

pub(crate) const RESULT_OK: NativeResult = 0;
pub(crate) const RESULT_INVALID_ARGUMENT: NativeResult = 1;
pub(crate) const RESULT_ABI_MISMATCH: NativeResult = 2;
pub(crate) const RESULT_ALLOCATION_FAILED: NativeResult = 3;
pub(crate) const RESULT_INTERNAL_ERROR: NativeResult = 4;
pub(crate) const RESULT_UNAVAILABLE: NativeResult = 5;
pub(crate) const RESULT_WRONG_THREAD: NativeResult = 6;
pub(crate) const RESULT_CALLBACK_IN_FLIGHT: NativeResult = 7;
pub(crate) const RESULT_INVALID_STATE: NativeResult = 8;
pub(crate) const RESULT_PRESENTATION_INCOMPLETE: NativeResult = 9;
#[allow(dead_code)]
pub(crate) const CREDENTIAL_LEGACY_SIZE: u32 = if cfg!(target_pointer_width = "64") {
    48
} else {
    28
};
#[allow(dead_code)]
pub(crate) const CONNECTION_LEGACY_SIZE: u32 = if cfg!(target_pointer_width = "64") {
    48
} else {
    36
};
#[allow(dead_code)]
pub(crate) const LAST_ERROR_LEGACY_SIZE: u32 = 24;

#[allow(dead_code)]
pub(crate) const CREATE_STAGE_NONE: u32 = 0;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_OLE_INITIALIZE: u32 = 1;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_ATL_AX_WIN_INIT: u32 = 2;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_CREATE_WINDOW: u32 = 3;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_CREATE_CONTROL: u32 = 4;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_QUERY_CLIENT: u32 = 5;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_QUERY_NON_SCRIPTABLE: u32 = 6;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_SET_PARENT: u32 = 7;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_EVENT_SUBSCRIPTION: u32 = 8;
#[allow(dead_code)]
pub(crate) const CREATE_STAGE_EXCEPTION: u32 = 9;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_GET_CONNECTED: u32 = 10;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_SET_SERVER: u32 = 11;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_GET_ADVANCED_SETTINGS: u32 = 12;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_SET_RDP_PORT: u32 = 13;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_POLICY: u32 = 14;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_SET_DESKTOP_WIDTH: u32 = 15;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_SET_DESKTOP_HEIGHT: u32 = 16;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_SET_COLOR_DEPTH: u32 = 17;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_INVOKE: u32 = 18;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_DISPLAY_DESKTOP_SCALE_FACTOR: u32 = 19;
#[allow(dead_code)]
pub(crate) const STAGE_CONNECT_DISPLAY_DEVICE_SCALE_FACTOR: u32 = 20;

pub(crate) const CONNECTION_STATE_DISCONNECTED: u32 = 0;
pub(crate) const CONNECTION_STATE_CONNECTED: u32 = 1;
pub(crate) const CONNECTION_STATE_CONNECTING: u32 = 2;
pub(crate) const REQUEST_CLOSE_CAN_PROCEED: u32 = 0;
pub(crate) const REQUEST_CLOSE_WAIT_FOR_EVENTS: u32 = 1;

// Shared reconnect bounds. MAX_RECONNECT_ATTEMPTS mirrors
// LINOP_RDP_MAX_RECONNECT_ATTEMPTS; MAX_KEEP_ALIVE_SECONDS keeps the
// seconds→milliseconds LONG conversion in range.
pub(crate) const MAX_RECONNECT_ATTEMPTS: u32 = 200;
pub(crate) const MAX_KEEP_ALIVE_SECONDS: u32 = (i32::MAX / 1_000) as u32;
pub(crate) const PRESENTATION_STATE_ABI_VERSION: u32 = 1;

pub(crate) const EVENT_CONNECTING: u32 = 1;
pub(crate) const EVENT_CONNECTED: u32 = 2;
pub(crate) const EVENT_LOGIN_COMPLETE: u32 = 3;
pub(crate) const EVENT_RECONNECTING: u32 = 4;
pub(crate) const EVENT_RECONNECTED: u32 = 5;
pub(crate) const EVENT_NETWORK_STATUS_CHANGED: u32 = 6;
pub(crate) const EVENT_REMOTE_DESKTOP_SIZE_CHANGED: u32 = 7;
pub(crate) const EVENT_ENTER_FULLSCREEN: u32 = 8;
pub(crate) const EVENT_LEAVE_FULLSCREEN: u32 = 9;
pub(crate) const EVENT_AUTHENTICATION_WARNING_DISPLAYED: u32 = 10;
pub(crate) const EVENT_AUTHENTICATION_WARNING_DISMISSED: u32 = 11;
pub(crate) const EVENT_WARNING: u32 = 12;
pub(crate) const EVENT_FATAL_ERROR: u32 = 13;
pub(crate) const EVENT_LOGON_ERROR: u32 = 14;
pub(crate) const EVENT_DISCONNECTED: u32 = 15;
pub(crate) const EVENT_CLOSE_CONFIRMED: u32 = 16;
pub(crate) const EVENT_FOCUS_RELEASED: u32 = 17;
pub(crate) const MAX_EVENT_PAYLOAD_BYTES: u32 = 65_536;

#[repr(C)]
pub(crate) struct NativeRdpHost {
    _private: [u8; 0],
    _marker: PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

#[repr(C)]
pub(crate) struct LinopRdpProbeOptions {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
}

impl LinopRdpProbeOptions {
    pub(crate) fn current() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
        }
    }
}

#[repr(C)]
pub(crate) struct LinopRdpProbeResult {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) available: u32,
    pub(crate) reserved: u32,
}

impl LinopRdpProbeResult {
    pub(crate) fn current() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            available: 0,
            reserved: 0,
        }
    }

    pub(crate) fn has_current_layout(&self) -> bool {
        self.struct_size == size_of::<Self>() as u32
            && self.abi_version == ABI_VERSION
            && self.reserved == 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinopRdpLastError {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) result: NativeResult,
    pub(crate) hresult: i32,
    pub(crate) has_hresult: u32,
    pub(crate) reserved: u32,
    pub(crate) stage: u32,
    pub(crate) win32_code: u32,
    pub(crate) has_win32_code: u32,
}

impl LinopRdpLastError {
    pub(crate) fn current() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            result: RESULT_OK,
            hresult: 0,
            has_hresult: 0,
            reserved: 0,
            stage: CREATE_STAGE_NONE,
            win32_code: 0,
            has_win32_code: 0,
        }
    }

    pub(crate) fn has_current_layout(&self) -> bool {
        self.struct_size >= size_of::<Self>() as u32
            && self.abi_version == ABI_VERSION
            && self.has_hresult <= 1
            && self.has_win32_code <= 1
            && self.reserved == 0
    }
}

#[repr(C)]
pub(crate) struct LinopRdpCreateOptions {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) generation_low: u32,
    pub(crate) generation_high: u32,
}

impl LinopRdpCreateOptions {
    pub(crate) fn current(generation: u64) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            generation_low: generation as u32,
            generation_high: (generation >> 32) as u32,
        }
    }
}

#[repr(C)]
pub(crate) struct LinopRdpCreateWithParentOptions {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) generation_low: u32,
    pub(crate) generation_high: u32,
    pub(crate) parent_hwnd: usize,
}

impl LinopRdpCreateWithParentOptions {
    pub(crate) fn current(generation: u64, parent_hwnd: usize) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: CREATE_WITH_PARENT_ABI_VERSION,
            generation_low: generation as u32,
            generation_high: (generation >> 32) as u32,
            parent_hwnd,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinopRdpBounds {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl LinopRdpBounds {
    pub(crate) const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinopRdpSessionDisplaySettings {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) desktop_width: u32,
    pub(crate) desktop_height: u32,
    pub(crate) physical_width: u32,
    pub(crate) physical_height: u32,
    pub(crate) orientation: u32,
    pub(crate) desktop_scale_factor: u32,
    pub(crate) device_scale_factor: u32,
}

impl LinopRdpSessionDisplaySettings {
    pub(crate) const fn current(
        desktop_width: u32,
        desktop_height: u32,
        physical_width: u32,
        physical_height: u32,
        orientation: u32,
        desktop_scale_factor: u32,
        device_scale_factor: u32,
    ) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: SESSION_DISPLAY_SETTINGS_ABI_VERSION,
            desktop_width,
            desktop_height,
            physical_width,
            physical_height,
            orientation,
            desktop_scale_factor,
            device_scale_factor,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinopRdpPresentationState {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) control_hwnd_valid: u32,
    pub(crate) host_rect_nonzero: u32,
    pub(crate) control_rect_nonzero: u32,
    pub(crate) control_visible: u32,
    pub(crate) control_is_host_descendant: u32,
    pub(crate) host_visible: u32,
}

impl LinopRdpPresentationState {
    pub(crate) const fn current() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: PRESENTATION_STATE_ABI_VERSION,
            control_hwnd_valid: 0,
            host_rect_nonzero: 0,
            control_rect_nonzero: 0,
            control_visible: 0,
            control_is_host_descendant: 0,
            host_visible: 0,
        }
    }

    pub(crate) fn has_current_layout(&self) -> bool {
        self.struct_size == size_of::<Self>() as u32
            && self.abi_version == PRESENTATION_STATE_ABI_VERSION
            && self.control_hwnd_valid <= 1
            && self.host_rect_nonzero <= 1
            && self.control_rect_nonzero <= 1
            && self.control_visible <= 1
            && self.control_is_host_descendant <= 1
            && self.host_visible <= 1
    }
}

#[repr(C)]
pub(crate) struct LinopRdpEvent {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) kind: u32,
    pub(crate) reserved: u32,
    pub(crate) generation_low: u32,
    pub(crate) generation_high: u32,
    pub(crate) code: i32,
    pub(crate) payload_len: u32,
}

impl LinopRdpEvent {
    #[cfg(test)]
    pub(crate) fn current(generation: u64, kind: u32, code: i32, payload_len: u32) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            kind,
            reserved: 0,
            generation_low: generation as u32,
            generation_high: (generation >> 32) as u32,
            code,
            payload_len,
        }
    }
}

#[repr(C)]
pub(crate) struct LinopRdpEventCallbackOptions {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) generation_low: u32,
    pub(crate) generation_high: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinopRdpBorrowedUtf16 {
    pub(crate) data: *const u16,
    pub(crate) len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinopRdpBorrowedSecret {
    pub(crate) data: *const u16,
    pub(crate) len: u32,
}

#[repr(C)]
pub(crate) struct LinopRdpCredentialBundle {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) server_password: LinopRdpBorrowedSecret,
    pub(crate) gateway_password: LinopRdpBorrowedSecret,
    pub(crate) flags: u32,
    pub(crate) username: LinopRdpBorrowedUtf16,
    pub(crate) domain: LinopRdpBorrowedUtf16,
    pub(crate) gateway_username: LinopRdpBorrowedUtf16,
    pub(crate) gateway_domain: LinopRdpBorrowedUtf16,
}

#[repr(C)]
pub(crate) struct LinopRdpConnectionOptions {
    pub(crate) struct_size: u32,
    pub(crate) abi_version: u32,
    pub(crate) host: LinopRdpBorrowedUtf16,
    pub(crate) port: u32,
    pub(crate) desktop_width: i32,
    pub(crate) desktop_height: i32,
    pub(crate) color_depth: i32,
    pub(crate) flags: u32,
    pub(crate) legacy_reserved: u32,
    pub(crate) display_mode: u32,
    pub(crate) display_flags: u32,
    pub(crate) desktop_scale_factor: u32,
    pub(crate) device_scale_factor: u32,
    pub(crate) resource_flags: u32,
    pub(crate) audio_mode: u32,
    pub(crate) audio_quality: u32,
    pub(crate) audio_flags: u32,
    pub(crate) keyboard_hook_mode: u32,
    pub(crate) input_flags: u32,
    pub(crate) performance_preset: u32,
    pub(crate) performance_flags: u32,
    pub(crate) network_connection_type: u32,
    pub(crate) security_flags: u32,
    pub(crate) authentication_level: u32,
    pub(crate) gateway_mode: u32,
    pub(crate) gateway_flags: u32,
    pub(crate) gateway_credential_source: u32,
    pub(crate) gateway_hostname: LinopRdpBorrowedUtf16,
    pub(crate) keep_alive_seconds: u32,
    pub(crate) timeout_seconds: u32,
    pub(crate) connection_flags: u32,
    pub(crate) max_reconnect_attempts: u32,
}

impl LinopRdpConnectionOptions {
    pub(crate) fn current(
        host: LinopRdpBorrowedUtf16,
        port: u32,
        desktop_width: i32,
        desktop_height: i32,
        color_depth: i32,
    ) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            host,
            port,
            desktop_width,
            desktop_height,
            color_depth,
            flags: 0,
            legacy_reserved: 0,
            display_mode: 0,
            display_flags: 0,
            desktop_scale_factor: 100,
            device_scale_factor: 100,
            resource_flags: RESOURCE_FLAG_CLIPBOARD,
            audio_mode: 0,
            audio_quality: 0,
            audio_flags: 0,
            keyboard_hook_mode: 1,
            input_flags: INPUT_FLAGS_KNOWN,
            performance_preset: 0,
            performance_flags: PERFORMANCE_FLAGS_KNOWN,
            network_connection_type: 6,
            security_flags: SECURITY_FLAG_ENABLE_CREDSSP | SECURITY_FLAG_ENCRYPTION_ENABLED,
            authentication_level: 0,
            gateway_mode: 0,
            gateway_flags: GATEWAY_FLAG_BYPASS_LOCAL,
            gateway_credential_source: 0,
            gateway_hostname: LinopRdpBorrowedUtf16 {
                data: std::ptr::null(),
                len: 0,
            },
            keep_alive_seconds: 60,
            timeout_seconds: 600,
            connection_flags: CONNECTION_FLAG_AUTO_RECONNECT,
            max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS,
        }
    }
}

#[allow(dead_code)]
fn abi_field_available<T>(struct_size: u32, offset: usize) -> bool {
    offset
        .checked_add(size_of::<T>())
        .is_some_and(|end| end <= struct_size as usize)
}

#[allow(dead_code)]
unsafe fn read_abi_field<T: Copy>(
    base: *const c_void,
    struct_size: u32,
    offset: usize,
) -> Option<T> {
    abi_field_available::<T>(struct_size, offset)
        .then(|| unsafe { std::ptr::read_unaligned(base.cast::<u8>().add(offset).cast::<T>()) })
}

#[allow(dead_code)]
fn valid_borrowed_utf16(value: LinopRdpBorrowedUtf16) -> bool {
    value.len == 0 || !value.data.is_null()
}

#[allow(dead_code)]
fn valid_borrowed_secret(value: LinopRdpBorrowedSecret) -> bool {
    value.len == 0 || !value.data.is_null()
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn normalize_connection_options(
    options: *const LinopRdpConnectionOptions,
) -> Result<LinopRdpConnectionOptions, NativeResult> {
    let base = options.cast::<c_void>();
    let struct_size = unsafe { read_abi_field::<u32>(base, size_of::<u32>() as u32, 0) }
        .ok_or(RESULT_INVALID_ARGUMENT)?;
    if struct_size < CONNECTION_LEGACY_SIZE {
        return Err(RESULT_INVALID_ARGUMENT);
    }

    let read_required = |offset| unsafe {
        read_abi_field::<u32>(base, struct_size, offset).ok_or(RESULT_INVALID_ARGUMENT)
    };
    let abi_version = read_required(std::mem::offset_of!(LinopRdpConnectionOptions, abi_version))?;
    if abi_version != ABI_VERSION {
        return Err(RESULT_ABI_MISMATCH);
    }

    let host = unsafe {
        read_abi_field::<LinopRdpBorrowedUtf16>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpConnectionOptions, host),
        )
    }
    .ok_or(RESULT_INVALID_ARGUMENT)?;
    let port = read_required(std::mem::offset_of!(LinopRdpConnectionOptions, port))?;
    let desktop_width = unsafe {
        read_abi_field::<i32>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpConnectionOptions, desktop_width),
        )
    }
    .ok_or(RESULT_INVALID_ARGUMENT)?;
    let desktop_height = unsafe {
        read_abi_field::<i32>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpConnectionOptions, desktop_height),
        )
    }
    .ok_or(RESULT_INVALID_ARGUMENT)?;
    let color_depth = unsafe {
        read_abi_field::<i32>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpConnectionOptions, color_depth),
        )
    }
    .ok_or(RESULT_INVALID_ARGUMENT)?;
    let flags = read_required(std::mem::offset_of!(LinopRdpConnectionOptions, flags))?;

    let mut normalized =
        LinopRdpConnectionOptions::current(host, port, desktop_width, desktop_height, color_depth);
    normalized.flags = flags;

    macro_rules! copy_optional_field {
        ($field:ident) => {
            if let Some(value) = unsafe {
                read_abi_field(
                    base,
                    struct_size,
                    std::mem::offset_of!(LinopRdpConnectionOptions, $field),
                )
            } {
                normalized.$field = value;
            }
        };
    }

    copy_optional_field!(display_mode);
    copy_optional_field!(display_flags);
    copy_optional_field!(desktop_scale_factor);
    copy_optional_field!(device_scale_factor);
    copy_optional_field!(resource_flags);
    let audio_mode_available = abi_field_available::<u32>(
        struct_size,
        std::mem::offset_of!(LinopRdpConnectionOptions, audio_mode),
    );
    copy_optional_field!(audio_mode);
    copy_optional_field!(audio_quality);
    copy_optional_field!(audio_flags);
    copy_optional_field!(keyboard_hook_mode);
    copy_optional_field!(input_flags);
    copy_optional_field!(performance_preset);
    copy_optional_field!(performance_flags);
    copy_optional_field!(network_connection_type);
    copy_optional_field!(security_flags);
    copy_optional_field!(authentication_level);
    copy_optional_field!(gateway_mode);
    copy_optional_field!(gateway_flags);
    copy_optional_field!(gateway_credential_source);
    copy_optional_field!(gateway_hostname);
    copy_optional_field!(keep_alive_seconds);
    copy_optional_field!(timeout_seconds);
    copy_optional_field!(connection_flags);
    copy_optional_field!(max_reconnect_attempts);

    if !audio_mode_available && flags & CONNECTION_FLAG_AUDIO_PLAYBACK_DISABLED != 0 {
        normalized.audio_mode = 2;
    }

    Ok(normalized)
}

#[cfg(not(windows_rdp_host_native))]
fn validate_connection_options(options: &LinopRdpConnectionOptions) -> NativeResult {
    if options.flags & !CONNECTION_FLAGS_KNOWN != 0
        || options.display_flags & !DISPLAY_FLAGS_KNOWN != 0
        || options.resource_flags & !RESOURCE_FLAGS_KNOWN != 0
        || options.audio_flags & !AUDIO_FLAGS_KNOWN != 0
        || options.input_flags & !INPUT_FLAGS_KNOWN != 0
        || options.performance_flags & !PERFORMANCE_FLAGS_KNOWN != 0
        || options.security_flags & !SECURITY_FLAGS_KNOWN != 0
        || options.gateway_flags & !GATEWAY_FLAGS_KNOWN != 0
        || options.connection_flags & !CONNECTION_POLICY_FLAGS_KNOWN != 0
    {
        return RESULT_INVALID_ARGUMENT;
    }

    if !matches!(options.display_mode, 0 | 1)
        || !matches!(options.audio_mode, 0..=2)
        || !matches!(options.audio_quality, 0..=2)
        || !matches!(options.keyboard_hook_mode, 0..=2)
        || !(0..=4).contains(&options.performance_preset)
        || !(0..=6).contains(&options.network_connection_type)
        || !(0..=2).contains(&options.authentication_level)
        || !matches!(options.gateway_mode, 0..=2)
        || !matches!(options.gateway_credential_source, 0 | 1 | 4)
        || !(100..=500).contains(&options.desktop_scale_factor)
        || !matches!(options.device_scale_factor, 100 | 140 | 180)
        || options.keep_alive_seconds > MAX_KEEP_ALIVE_SECONDS
        || options.timeout_seconds > i32::MAX as u32
        || options.max_reconnect_attempts > MAX_RECONNECT_ATTEMPTS
    {
        return RESULT_INVALID_ARGUMENT;
    }

    if options.host.len == 0
        || options.host.len as usize > crate::options::WINDOWS_RDP_MAX_HOST_UTF16_CODE_UNITS
        || options.host.data.is_null()
    {
        return RESULT_INVALID_ARGUMENT;
    }
    let host_slice =
        unsafe { std::slice::from_raw_parts(options.host.data, options.host.len as usize) };
    if host_slice.contains(&0) {
        return RESULT_INVALID_ARGUMENT;
    }

    if !(1..=u32::from(u16::MAX)).contains(&options.port)
        || options.desktop_width <= 0
        || options.desktop_height <= 0
        || !matches!(options.color_depth, 8 | 15 | 16 | 24 | 32)
    {
        return RESULT_INVALID_ARGUMENT;
    }

    let gateway = options.gateway_hostname;
    if !valid_borrowed_utf16(gateway)
        || gateway.len as usize > crate::policy::WINDOWS_RDP_MAX_GATEWAY_HOST_UTF16_CODE_UNITS
        || (options.gateway_mode == 1 && gateway.len == 0)
    {
        return RESULT_INVALID_ARGUMENT;
    }
    if gateway.len > 0 {
        let gateway_slice =
            unsafe { std::slice::from_raw_parts(gateway.data, gateway.len as usize) };
        if gateway_slice.contains(&0) {
            return RESULT_INVALID_ARGUMENT;
        }
    }

    RESULT_OK
}

const _: () = {
    assert!(size_of::<NativeResult>() == 4);

    assert!(size_of::<LinopRdpProbeOptions>() == 8);
    assert!(align_of::<LinopRdpProbeOptions>() == 4);
    assert!(std::mem::offset_of!(LinopRdpProbeOptions, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpProbeOptions, abi_version) == 4);

    assert!(size_of::<LinopRdpProbeResult>() == 16);
    assert!(align_of::<LinopRdpProbeResult>() == 4);
    assert!(std::mem::offset_of!(LinopRdpProbeResult, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpProbeResult, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpProbeResult, available) == 8);
    assert!(std::mem::offset_of!(LinopRdpProbeResult, reserved) == 12);

    assert!(size_of::<LinopRdpLastError>() == 36);
    assert!(align_of::<LinopRdpLastError>() == 4);
    assert!(std::mem::offset_of!(LinopRdpLastError, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpLastError, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpLastError, result) == 8);
    assert!(std::mem::offset_of!(LinopRdpLastError, hresult) == 12);
    assert!(std::mem::offset_of!(LinopRdpLastError, has_hresult) == 16);
    assert!(std::mem::offset_of!(LinopRdpLastError, reserved) == 20);
    assert!(std::mem::offset_of!(LinopRdpLastError, stage) == 24);
    assert!(std::mem::offset_of!(LinopRdpLastError, win32_code) == 28);
    assert!(std::mem::offset_of!(LinopRdpLastError, has_win32_code) == 32);

    assert!(size_of::<LinopRdpCreateOptions>() == 16);
    assert!(align_of::<LinopRdpCreateOptions>() == 4);
    assert!(std::mem::offset_of!(LinopRdpCreateOptions, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpCreateOptions, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpCreateOptions, generation_low) == 8);
    assert!(std::mem::offset_of!(LinopRdpCreateOptions, generation_high) == 12);

    assert!(std::mem::offset_of!(LinopRdpCreateWithParentOptions, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpCreateWithParentOptions, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpCreateWithParentOptions, generation_low) == 8);
    assert!(std::mem::offset_of!(LinopRdpCreateWithParentOptions, generation_high) == 12);
    assert!(std::mem::offset_of!(LinopRdpCreateWithParentOptions, parent_hwnd) == 16);

    assert!(size_of::<LinopRdpBounds>() == 16);
    assert!(align_of::<LinopRdpBounds>() == 4);
    assert!(std::mem::offset_of!(LinopRdpBounds, x) == 0);
    assert!(std::mem::offset_of!(LinopRdpBounds, y) == 4);
    assert!(std::mem::offset_of!(LinopRdpBounds, width) == 8);
    assert!(std::mem::offset_of!(LinopRdpBounds, height) == 12);

    assert!(size_of::<LinopRdpSessionDisplaySettings>() == 36);
    assert!(align_of::<LinopRdpSessionDisplaySettings>() == 4);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_width) == 8);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_height) == 12);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, physical_width) == 16);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, physical_height) == 20);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, orientation) == 24);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_scale_factor) == 28);
    assert!(std::mem::offset_of!(LinopRdpSessionDisplaySettings, device_scale_factor) == 32);

    assert!(size_of::<LinopRdpEvent>() == 32);
    assert!(align_of::<LinopRdpEvent>() == 4);
    assert!(std::mem::offset_of!(LinopRdpEvent, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpEvent, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpEvent, kind) == 8);
    assert!(std::mem::offset_of!(LinopRdpEvent, reserved) == 12);
    assert!(std::mem::offset_of!(LinopRdpEvent, generation_low) == 16);
    assert!(std::mem::offset_of!(LinopRdpEvent, generation_high) == 20);
    assert!(std::mem::offset_of!(LinopRdpEvent, code) == 24);
    assert!(std::mem::offset_of!(LinopRdpEvent, payload_len) == 28);

    assert!(size_of::<LinopRdpEventCallbackOptions>() == 16);
    assert!(align_of::<LinopRdpEventCallbackOptions>() == 4);
    assert!(std::mem::offset_of!(LinopRdpEventCallbackOptions, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpEventCallbackOptions, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpEventCallbackOptions, generation_low) == 8);
    assert!(std::mem::offset_of!(LinopRdpEventCallbackOptions, generation_high) == 12);

    assert!(std::mem::offset_of!(LinopRdpBorrowedSecret, data) == 0);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, server_password) == 8);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, username) > 8);
    assert!(
        std::mem::offset_of!(LinopRdpCredentialBundle, domain)
            > std::mem::offset_of!(LinopRdpCredentialBundle, username)
    );
    assert!(
        std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username)
            > std::mem::offset_of!(LinopRdpCredentialBundle, domain)
    );
    assert!(
        std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain)
            > std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username)
    );
    assert!(std::mem::offset_of!(LinopRdpBorrowedUtf16, data) == 0);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, struct_size) == 0);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, abi_version) == 4);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, host) == 8);
};

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<LinopRdpCreateWithParentOptions>() == 24);
    assert!(align_of::<LinopRdpCreateWithParentOptions>() == 8);
    assert!(size_of::<LinopRdpBorrowedSecret>() == 16);
    assert!(align_of::<LinopRdpBorrowedSecret>() == 8);
    assert!(std::mem::offset_of!(LinopRdpBorrowedSecret, len) == 8);
    assert!(size_of::<LinopRdpCredentialBundle>() == 112);
    assert!(align_of::<LinopRdpCredentialBundle>() == 8);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_password) == 24);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, flags) == 40);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, username) == 48);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, domain) == 64);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username) == 80);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain) == 96);
    assert!(size_of::<LinopRdpBorrowedUtf16>() == 16);
    assert!(align_of::<LinopRdpBorrowedUtf16>() == 8);
    assert!(std::mem::offset_of!(LinopRdpBorrowedUtf16, len) == 8);
    assert!(size_of::<LinopRdpConnectionOptions>() == 152);
    assert!(align_of::<LinopRdpConnectionOptions>() == 8);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, port) == 24);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_width) == 28);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_height) == 32);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, color_depth) == 36);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, flags) == 40);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, legacy_reserved) == 44);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, display_mode) == 48);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, display_flags) == 52);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_scale_factor) == 56);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, device_scale_factor) == 60);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, resource_flags) == 64);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_mode) == 68);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_quality) == 72);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_flags) == 76);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, keyboard_hook_mode) == 80);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, input_flags) == 84);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, performance_preset) == 88);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, performance_flags) == 92);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, network_connection_type) == 96);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, security_flags) == 100);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, authentication_level) == 104);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_mode) == 108);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_flags) == 112);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_credential_source) == 116);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_hostname) == 120);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, keep_alive_seconds) == 136);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, timeout_seconds) == 140);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, connection_flags) == 144);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, max_reconnect_attempts) == 148);
};

#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(size_of::<LinopRdpCreateWithParentOptions>() == 20);
    assert!(align_of::<LinopRdpCreateWithParentOptions>() == 4);
    assert!(size_of::<LinopRdpBorrowedSecret>() == 8);
    assert!(align_of::<LinopRdpBorrowedSecret>() == 4);
    assert!(std::mem::offset_of!(LinopRdpBorrowedSecret, len) == 4);
    assert!(size_of::<LinopRdpCredentialBundle>() == 60);
    assert!(align_of::<LinopRdpCredentialBundle>() == 4);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_password) == 16);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, flags) == 24);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, username) == 28);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, domain) == 36);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username) == 44);
    assert!(std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain) == 52);
    assert!(size_of::<LinopRdpBorrowedUtf16>() == 8);
    assert!(align_of::<LinopRdpBorrowedUtf16>() == 4);
    assert!(std::mem::offset_of!(LinopRdpBorrowedUtf16, len) == 4);
    assert!(size_of::<LinopRdpConnectionOptions>() == 136);
    assert!(align_of::<LinopRdpConnectionOptions>() == 4);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, port) == 16);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_width) == 20);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_height) == 24);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, color_depth) == 28);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, flags) == 32);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, legacy_reserved) == 36);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, display_mode) == 40);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, display_flags) == 44);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, desktop_scale_factor) == 48);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, device_scale_factor) == 52);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, resource_flags) == 56);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_mode) == 60);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_quality) == 64);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, audio_flags) == 68);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, keyboard_hook_mode) == 72);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, input_flags) == 76);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, performance_preset) == 80);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, performance_flags) == 84);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, network_connection_type) == 88);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, security_flags) == 92);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, authentication_level) == 96);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_mode) == 100);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_flags) == 104);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_credential_source) == 108);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, gateway_hostname) == 112);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, keep_alive_seconds) == 120);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, timeout_seconds) == 124);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, connection_flags) == 128);
    assert!(std::mem::offset_of!(LinopRdpConnectionOptions, max_reconnect_attempts) == 132);
};

impl LinopRdpEventCallbackOptions {
    pub(crate) fn current(generation: u64) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            generation_low: generation as u32,
            generation_high: (generation >> 32) as u32,
        }
    }
}

pub(crate) type NativeEventCallback =
    unsafe extern "C" fn(context: *mut c_void, event: *const LinopRdpEvent, payload: *const u8);

pub(crate) type ProbeFn = unsafe fn(
    options: *const LinopRdpProbeOptions,
    out_result: *mut LinopRdpProbeResult,
) -> NativeResult;
pub(crate) type CreateFn = unsafe fn(
    options: *const LinopRdpCreateOptions,
    out_host: *mut *mut NativeRdpHost,
) -> NativeResult;
pub(crate) type CreateWithParentV2Fn = unsafe fn(
    options: *const LinopRdpCreateWithParentOptions,
    out_host: *mut *mut NativeRdpHost,
    out_error: *mut LinopRdpLastError,
) -> NativeResult;
pub(crate) type GetLastErrorFn =
    unsafe fn(host: *mut NativeRdpHost, out_error: *mut LinopRdpLastError) -> NativeResult;
pub(crate) type SetBoundsFn =
    unsafe fn(host: *mut NativeRdpHost, bounds: *const LinopRdpBounds) -> NativeResult;
pub(crate) type UpdateSessionDisplaySettingsFn = unsafe fn(
    host: *mut NativeRdpHost,
    settings: *const LinopRdpSessionDisplaySettings,
) -> NativeResult;
pub(crate) type GetPresentationStateFn =
    unsafe fn(host: *mut NativeRdpHost, out_state: *mut LinopRdpPresentationState) -> NativeResult;
pub(crate) type SetVisibleFn = unsafe fn(host: *mut NativeRdpHost, visible: u32) -> NativeResult;
pub(crate) type FocusFn = unsafe fn(host: *mut NativeRdpHost) -> NativeResult;
pub(crate) type DestroyFn = unsafe fn(host: *mut *mut NativeRdpHost) -> NativeResult;
pub(crate) type RegisterEventCallbackFn = unsafe fn(
    host: *mut NativeRdpHost,
    options: *const LinopRdpEventCallbackOptions,
    callback: Option<NativeEventCallback>,
    callback_context: *mut c_void,
) -> NativeResult;
pub(crate) type UnregisterEventCallbackFn = unsafe fn(host: *mut NativeRdpHost) -> NativeResult;
pub(crate) type ApplyCredentialsFn = unsafe fn(
    host: *mut NativeRdpHost,
    credentials: *const LinopRdpCredentialBundle,
) -> NativeResult;
pub(crate) type ConnectFn =
    unsafe fn(host: *mut NativeRdpHost, options: *const LinopRdpConnectionOptions) -> NativeResult;
pub(crate) type GetConnectionStateFn =
    unsafe fn(host: *mut NativeRdpHost, out_state: *mut u32) -> NativeResult;
pub(crate) type RequestCloseFn =
    unsafe fn(host: *mut NativeRdpHost, out_status: *mut u32) -> NativeResult;
pub(crate) type DisconnectFn = unsafe fn(host: *mut NativeRdpHost) -> NativeResult;

#[derive(Clone, Copy)]
pub(crate) struct NativeBindings {
    pub(crate) probe: ProbeFn,
    pub(crate) create: CreateFn,
    pub(crate) create_with_parent_v2: CreateWithParentV2Fn,
    pub(crate) get_last_error: GetLastErrorFn,
    pub(crate) set_bounds: SetBoundsFn,
    pub(crate) update_session_display_settings: UpdateSessionDisplaySettingsFn,
    pub(crate) get_presentation_state: GetPresentationStateFn,
    pub(crate) set_visible: SetVisibleFn,
    pub(crate) focus: FocusFn,
    pub(crate) destroy: DestroyFn,
    pub(crate) register_event_callback: RegisterEventCallbackFn,
    pub(crate) unregister_event_callback: UnregisterEventCallbackFn,
    pub(crate) apply_credentials: ApplyCredentialsFn,
    pub(crate) connect: ConnectFn,
    pub(crate) get_connection_state: GetConnectionStateFn,
    pub(crate) request_close: RequestCloseFn,
    pub(crate) disconnect: DisconnectFn,
}

pub(crate) const NATIVE_BINDINGS: NativeBindings = NativeBindings {
    probe,
    create,
    create_with_parent_v2,
    get_last_error,
    set_bounds,
    update_session_display_settings,
    get_presentation_state,
    set_visible,
    focus,
    destroy,
    register_event_callback,
    unregister_event_callback,
    apply_credentials,
    connect,
    get_connection_state,
    request_close,
    disconnect,
};

#[cfg(windows_rdp_host_native)]
unsafe extern "C" {
    fn linop_rdp_probe(
        options: *const LinopRdpProbeOptions,
        out_result: *mut LinopRdpProbeResult,
    ) -> NativeResult;
    fn linop_rdp_create(
        options: *const LinopRdpCreateOptions,
        out_host: *mut *mut NativeRdpHost,
    ) -> NativeResult;
    fn linop_rdp_create_with_parent_v2(
        options: *const LinopRdpCreateWithParentOptions,
        out_host: *mut *mut NativeRdpHost,
        out_error: *mut LinopRdpLastError,
    ) -> NativeResult;
    fn linop_rdp_get_last_error(
        host: *mut NativeRdpHost,
        out_error: *mut LinopRdpLastError,
    ) -> NativeResult;
    fn linop_rdp_set_bounds(
        host: *mut NativeRdpHost,
        bounds: *const LinopRdpBounds,
    ) -> NativeResult;
    fn linop_rdp_update_session_display_settings(
        host: *mut NativeRdpHost,
        settings: *const LinopRdpSessionDisplaySettings,
    ) -> NativeResult;
    fn linop_rdp_get_presentation_state(
        host: *mut NativeRdpHost,
        out_state: *mut LinopRdpPresentationState,
    ) -> NativeResult;
    fn linop_rdp_set_visible(host: *mut NativeRdpHost, visible: u32) -> NativeResult;
    fn linop_rdp_focus(host: *mut NativeRdpHost) -> NativeResult;
    fn linop_rdp_register_event_callback(
        host: *mut NativeRdpHost,
        options: *const LinopRdpEventCallbackOptions,
        callback: Option<NativeEventCallback>,
        callback_context: *mut c_void,
    ) -> NativeResult;
    fn linop_rdp_unregister_event_callback(host: *mut NativeRdpHost) -> NativeResult;
    fn linop_rdp_apply_credentials(
        host: *mut NativeRdpHost,
        credentials: *const LinopRdpCredentialBundle,
    ) -> NativeResult;
    fn linop_rdp_connect(
        host: *mut NativeRdpHost,
        options: *const LinopRdpConnectionOptions,
    ) -> NativeResult;
    fn linop_rdp_get_connection_state(
        host: *mut NativeRdpHost,
        out_state: *mut u32,
    ) -> NativeResult;
    fn linop_rdp_request_close(host: *mut NativeRdpHost, out_status: *mut u32) -> NativeResult;
    fn linop_rdp_disconnect(host: *mut NativeRdpHost) -> NativeResult;
    fn linop_rdp_destroy(host: *mut *mut NativeRdpHost) -> NativeResult;
}

#[cfg(windows_rdp_host_native)]
unsafe fn probe(
    options: *const LinopRdpProbeOptions,
    out_result: *mut LinopRdpProbeResult,
) -> NativeResult {
    unsafe { linop_rdp_probe(options, out_result) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn create(
    options: *const LinopRdpCreateOptions,
    out_host: *mut *mut NativeRdpHost,
) -> NativeResult {
    unsafe { linop_rdp_create(options, out_host) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn create_with_parent_v2(
    options: *const LinopRdpCreateWithParentOptions,
    out_host: *mut *mut NativeRdpHost,
    out_error: *mut LinopRdpLastError,
) -> NativeResult {
    unsafe { linop_rdp_create_with_parent_v2(options, out_host, out_error) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn get_last_error(
    host: *mut NativeRdpHost,
    out_error: *mut LinopRdpLastError,
) -> NativeResult {
    unsafe { linop_rdp_get_last_error(host, out_error) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn set_bounds(host: *mut NativeRdpHost, bounds: *const LinopRdpBounds) -> NativeResult {
    unsafe { linop_rdp_set_bounds(host, bounds) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn update_session_display_settings(
    host: *mut NativeRdpHost,
    settings: *const LinopRdpSessionDisplaySettings,
) -> NativeResult {
    unsafe { linop_rdp_update_session_display_settings(host, settings) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn get_presentation_state(
    host: *mut NativeRdpHost,
    out_state: *mut LinopRdpPresentationState,
) -> NativeResult {
    unsafe { linop_rdp_get_presentation_state(host, out_state) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn set_visible(host: *mut NativeRdpHost, visible: u32) -> NativeResult {
    unsafe { linop_rdp_set_visible(host, visible) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn focus(host: *mut NativeRdpHost) -> NativeResult {
    unsafe { linop_rdp_focus(host) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn register_event_callback(
    host: *mut NativeRdpHost,
    options: *const LinopRdpEventCallbackOptions,
    callback: Option<NativeEventCallback>,
    callback_context: *mut c_void,
) -> NativeResult {
    unsafe { linop_rdp_register_event_callback(host, options, callback, callback_context) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn unregister_event_callback(host: *mut NativeRdpHost) -> NativeResult {
    unsafe { linop_rdp_unregister_event_callback(host) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn apply_credentials(
    host: *mut NativeRdpHost,
    credentials: *const LinopRdpCredentialBundle,
) -> NativeResult {
    unsafe { linop_rdp_apply_credentials(host, credentials) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn connect(
    host: *mut NativeRdpHost,
    options: *const LinopRdpConnectionOptions,
) -> NativeResult {
    unsafe { linop_rdp_connect(host, options) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn get_connection_state(host: *mut NativeRdpHost, out_state: *mut u32) -> NativeResult {
    unsafe { linop_rdp_get_connection_state(host, out_state) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn request_close(host: *mut NativeRdpHost, out_status: *mut u32) -> NativeResult {
    unsafe { linop_rdp_request_close(host, out_status) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn disconnect(host: *mut NativeRdpHost) -> NativeResult {
    unsafe { linop_rdp_disconnect(host) }
}

#[cfg(windows_rdp_host_native)]
unsafe fn destroy(host: *mut *mut NativeRdpHost) -> NativeResult {
    unsafe { linop_rdp_destroy(host) }
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn probe(
    options: *const LinopRdpProbeOptions,
    out_result: *mut LinopRdpProbeResult,
) -> NativeResult {
    if options.is_null() || out_result.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }

    let options_struct_size = unsafe { std::ptr::addr_of!((*options).struct_size).read() };
    if options_struct_size < size_of::<LinopRdpProbeOptions>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let options_abi_version = unsafe { std::ptr::addr_of!((*options).abi_version).read() };
    if options_abi_version != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }

    let caller_result_size = unsafe { std::ptr::addr_of!((*out_result).struct_size).read() };
    if caller_result_size < size_of::<LinopRdpProbeResult>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let result_abi_version = unsafe { std::ptr::addr_of!((*out_result).abi_version).read() };
    if result_abi_version != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }

    unsafe {
        *out_result = LinopRdpProbeResult {
            struct_size: caller_result_size,
            abi_version: ABI_VERSION,
            available: 0,
            reserved: 0,
        };
    }
    RESULT_OK
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn create(
    options: *const LinopRdpCreateOptions,
    out_host: *mut *mut NativeRdpHost,
) -> NativeResult {
    if out_host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    unsafe {
        *out_host = std::ptr::null_mut();
    }
    if options.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }

    let options_struct_size = unsafe { std::ptr::addr_of!((*options).struct_size).read() };
    if options_struct_size < size_of::<LinopRdpCreateOptions>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let options_abi_version = unsafe { std::ptr::addr_of!((*options).abi_version).read() };
    if options_abi_version != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }

    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn create_with_parent_v2(
    options: *const LinopRdpCreateWithParentOptions,
    out_host: *mut *mut NativeRdpHost,
    out_error: *mut LinopRdpLastError,
) -> NativeResult {
    if out_error.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    let error_size = unsafe { std::ptr::addr_of!((*out_error).struct_size).read() };
    if error_size < LAST_ERROR_LEGACY_SIZE {
        return RESULT_INVALID_ARGUMENT;
    }
    let error_abi = unsafe { std::ptr::addr_of!((*out_error).abi_version).read() };
    if error_abi != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }
    unsafe {
        (*out_error).struct_size = error_size;
        (*out_error).abi_version = ABI_VERSION;
        (*out_error).result = RESULT_OK;
        (*out_error).hresult = 0;
        (*out_error).has_hresult = 0;
        (*out_error).reserved = 0;
        if error_size
            >= std::mem::offset_of!(LinopRdpLastError, stage) as u32 + size_of::<u32>() as u32
        {
            (*out_error).stage = CREATE_STAGE_NONE;
        }
        if error_size
            >= std::mem::offset_of!(LinopRdpLastError, win32_code) as u32 + size_of::<u32>() as u32
        {
            (*out_error).win32_code = 0;
        }
        if error_size
            >= std::mem::offset_of!(LinopRdpLastError, has_win32_code) as u32
                + size_of::<u32>() as u32
        {
            (*out_error).has_win32_code = 0;
        }
    }
    if out_host.is_null() {
        unsafe { (*out_error).result = RESULT_INVALID_ARGUMENT };
        return RESULT_INVALID_ARGUMENT;
    }
    unsafe {
        *out_host = std::ptr::null_mut();
    }
    if options.is_null() {
        unsafe { (*out_error).result = RESULT_INVALID_ARGUMENT };
        return RESULT_INVALID_ARGUMENT;
    }

    let options_struct_size = unsafe { std::ptr::addr_of!((*options).struct_size).read() };
    if options_struct_size < size_of::<LinopRdpCreateWithParentOptions>() as u32 {
        unsafe { (*out_error).result = RESULT_INVALID_ARGUMENT };
        return RESULT_INVALID_ARGUMENT;
    }
    let options_abi_version = unsafe { std::ptr::addr_of!((*options).abi_version).read() };
    if options_abi_version != CREATE_WITH_PARENT_ABI_VERSION {
        unsafe { (*out_error).result = RESULT_ABI_MISMATCH };
        return RESULT_ABI_MISMATCH;
    }
    if unsafe { std::ptr::addr_of!((*options).parent_hwnd).read() } == 0 {
        unsafe { (*out_error).result = RESULT_INVALID_ARGUMENT };
        return RESULT_INVALID_ARGUMENT;
    }

    unsafe { (*out_error).result = RESULT_UNAVAILABLE };
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn get_last_error(
    host: *mut NativeRdpHost,
    out_error: *mut LinopRdpLastError,
) -> NativeResult {
    if host.is_null() || out_error.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn set_bounds(host: *mut NativeRdpHost, bounds: *const LinopRdpBounds) -> NativeResult {
    if host.is_null() || bounds.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }

    let width = unsafe { std::ptr::addr_of!((*bounds).width).read() };
    let height = unsafe { std::ptr::addr_of!((*bounds).height).read() };
    if width < 0 || height < 0 {
        return RESULT_INVALID_ARGUMENT;
    }

    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn update_session_display_settings(
    host: *mut NativeRdpHost,
    settings: *const LinopRdpSessionDisplaySettings,
) -> NativeResult {
    if host.is_null() || settings.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    let struct_size = unsafe { std::ptr::addr_of!((*settings).struct_size).read() };
    if struct_size < size_of::<LinopRdpSessionDisplaySettings>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let abi_version = unsafe { std::ptr::addr_of!((*settings).abi_version).read() };
    if abi_version != SESSION_DISPLAY_SETTINGS_ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }
    let desktop_width = unsafe { std::ptr::addr_of!((*settings).desktop_width).read() };
    let desktop_height = unsafe { std::ptr::addr_of!((*settings).desktop_height).read() };
    let physical_width = unsafe { std::ptr::addr_of!((*settings).physical_width).read() };
    let physical_height = unsafe { std::ptr::addr_of!((*settings).physical_height).read() };
    let desktop_scale_factor =
        unsafe { std::ptr::addr_of!((*settings).desktop_scale_factor).read() };
    let device_scale_factor = unsafe { std::ptr::addr_of!((*settings).device_scale_factor).read() };
    if desktop_width == 0
        || desktop_height == 0
        || physical_width == 0
        || physical_height == 0
        || desktop_scale_factor == 0
        || device_scale_factor == 0
    {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn set_visible(host: *mut NativeRdpHost, visible: u32) -> NativeResult {
    if host.is_null() || visible > 1 {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn get_presentation_state(
    host: *mut NativeRdpHost,
    out_state: *mut LinopRdpPresentationState,
) -> NativeResult {
    if host.is_null() || out_state.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    let struct_size = unsafe { std::ptr::addr_of!((*out_state).struct_size).read() };
    if struct_size < size_of::<LinopRdpPresentationState>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let abi_version = unsafe { std::ptr::addr_of!((*out_state).abi_version).read() };
    if abi_version != PRESENTATION_STATE_ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn focus(host: *mut NativeRdpHost) -> NativeResult {
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn register_event_callback(
    host: *mut NativeRdpHost,
    options: *const LinopRdpEventCallbackOptions,
    callback: Option<NativeEventCallback>,
    _callback_context: *mut c_void,
) -> NativeResult {
    if host.is_null() || options.is_null() || callback.is_none() {
        return RESULT_INVALID_ARGUMENT;
    }

    let options_struct_size = unsafe { std::ptr::addr_of!((*options).struct_size).read() };
    if options_struct_size < size_of::<LinopRdpEventCallbackOptions>() as u32 {
        return RESULT_INVALID_ARGUMENT;
    }
    let options_abi_version = unsafe { std::ptr::addr_of!((*options).abi_version).read() };
    if options_abi_version != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }

    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn unregister_event_callback(host: *mut NativeRdpHost) -> NativeResult {
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_OK
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn apply_credentials(
    host: *mut NativeRdpHost,
    credentials: *const LinopRdpCredentialBundle,
) -> NativeResult {
    if host.is_null() || credentials.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }

    let base = credentials.cast::<c_void>();
    let Some(struct_size) = (unsafe { read_abi_field::<u32>(base, size_of::<u32>() as u32, 0) })
    else {
        return RESULT_INVALID_ARGUMENT;
    };
    if struct_size < CREDENTIAL_LEGACY_SIZE {
        return RESULT_INVALID_ARGUMENT;
    }

    let read_required_u32 = |offset| unsafe {
        read_abi_field::<u32>(base, struct_size, offset).ok_or(RESULT_INVALID_ARGUMENT)
    };
    let abi_version =
        match read_required_u32(std::mem::offset_of!(LinopRdpCredentialBundle, abi_version)) {
            Ok(value) => value,
            Err(result) => return result,
        };
    if abi_version != ABI_VERSION {
        return RESULT_ABI_MISMATCH;
    }
    let flags = match read_required_u32(std::mem::offset_of!(LinopRdpCredentialBundle, flags)) {
        Ok(value) => value,
        Err(result) => return result,
    };
    if flags != 0 {
        return RESULT_INVALID_ARGUMENT;
    }

    let server_password = match unsafe {
        read_abi_field::<LinopRdpBorrowedSecret>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpCredentialBundle, server_password),
        )
        .ok_or(RESULT_INVALID_ARGUMENT)
    } {
        Ok(value) => value,
        Err(result) => return result,
    };
    if !valid_borrowed_secret(server_password) {
        return RESULT_INVALID_ARGUMENT;
    }
    let gateway_password = match unsafe {
        read_abi_field::<LinopRdpBorrowedSecret>(
            base,
            struct_size,
            std::mem::offset_of!(LinopRdpCredentialBundle, gateway_password),
        )
        .ok_or(RESULT_INVALID_ARGUMENT)
    } {
        Ok(value) => value,
        Err(result) => return result,
    };
    if !valid_borrowed_secret(gateway_password) {
        return RESULT_INVALID_ARGUMENT;
    }

    macro_rules! validate_optional_identity {
        ($field:ident) => {
            if let Some(value) = unsafe {
                read_abi_field::<LinopRdpBorrowedUtf16>(
                    base,
                    struct_size,
                    std::mem::offset_of!(LinopRdpCredentialBundle, $field),
                )
            } {
                if !valid_borrowed_utf16(value) {
                    return RESULT_INVALID_ARGUMENT;
                }
            }
        };
    }

    validate_optional_identity!(username);
    validate_optional_identity!(domain);
    validate_optional_identity!(gateway_username);
    validate_optional_identity!(gateway_domain);

    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn connect(
    host: *mut NativeRdpHost,
    options: *const LinopRdpConnectionOptions,
) -> NativeResult {
    if host.is_null() || options.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }

    let normalized = match unsafe { normalize_connection_options(options) } {
        Ok(options) => options,
        Err(result) => return result,
    };
    let validation_result = validate_connection_options(&normalized);
    if validation_result != RESULT_OK {
        return validation_result;
    }

    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn get_connection_state(host: *mut NativeRdpHost, out_state: *mut u32) -> NativeResult {
    if out_state.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    unsafe {
        *out_state = CONNECTION_STATE_DISCONNECTED;
    }
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn request_close(host: *mut NativeRdpHost, out_status: *mut u32) -> NativeResult {
    if out_status.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    unsafe {
        *out_status = REQUEST_CLOSE_CAN_PROCEED;
    }
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn disconnect(host: *mut NativeRdpHost) -> NativeResult {
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    RESULT_UNAVAILABLE
}

#[cfg(not(windows_rdp_host_native))]
unsafe fn destroy(host: *mut *mut NativeRdpHost) -> NativeResult {
    if host.is_null() {
        return RESULT_INVALID_ARGUMENT;
    }
    unsafe {
        *host = std::ptr::null_mut();
    }
    RESULT_OK
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn abi_constants_match_the_native_header() {
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(RESULT_OK, 0);
        assert_eq!(RESULT_INVALID_ARGUMENT, 1);
        assert_eq!(RESULT_ABI_MISMATCH, 2);
        assert_eq!(RESULT_ALLOCATION_FAILED, 3);
        assert_eq!(RESULT_INTERNAL_ERROR, 4);
        assert_eq!(RESULT_UNAVAILABLE, 5);
        assert_eq!(RESULT_WRONG_THREAD, 6);
        assert_eq!(RESULT_CALLBACK_IN_FLIGHT, 7);
        assert_eq!(RESULT_INVALID_STATE, 8);
        assert_eq!(size_of::<NativeResult>(), 4);
    }

    #[test]
    fn fixed_width_abi_struct_layout_is_architecture_independent() {
        assert_eq!(size_of::<LinopRdpProbeOptions>(), 8);
        assert_eq!(align_of::<LinopRdpProbeOptions>(), 4);
        assert_eq!(size_of::<LinopRdpProbeResult>(), 16);
        assert_eq!(align_of::<LinopRdpProbeResult>(), 4);
        assert_eq!(size_of::<LinopRdpLastError>(), 36);
        assert_eq!(align_of::<LinopRdpLastError>(), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, struct_size), 0);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, abi_version), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, result), 8);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, hresult), 12);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, has_hresult), 16);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, reserved), 20);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, stage), 24);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, win32_code), 28);
        assert_eq!(std::mem::offset_of!(LinopRdpLastError, has_win32_code), 32);
        assert_eq!(size_of::<LinopRdpCreateOptions>(), 16);
        assert_eq!(align_of::<LinopRdpCreateOptions>(), 4);
        assert_eq!(size_of::<LinopRdpBounds>(), 16);
        assert_eq!(align_of::<LinopRdpBounds>(), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpBounds, x), 0);
        assert_eq!(std::mem::offset_of!(LinopRdpBounds, y), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpBounds, width), 8);
        assert_eq!(std::mem::offset_of!(LinopRdpBounds, height), 12);
        assert_eq!(size_of::<LinopRdpSessionDisplaySettings>(), 36);
        assert_eq!(align_of::<LinopRdpSessionDisplaySettings>(), 4);
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, struct_size),
            0
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, abi_version),
            4
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_width),
            8
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_height),
            12
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, physical_width),
            16
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, physical_height),
            20
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, orientation),
            24
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, desktop_scale_factor),
            28
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpSessionDisplaySettings, device_scale_factor),
            32
        );
        assert_eq!(size_of::<LinopRdpEvent>(), 32);
        assert_eq!(align_of::<LinopRdpEvent>(), 4);
        assert_eq!(size_of::<LinopRdpEventCallbackOptions>(), 16);
        assert_eq!(align_of::<LinopRdpEventCallbackOptions>(), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, struct_size), 0);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, abi_version), 4);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, kind), 8);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, reserved), 12);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, generation_low), 16);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, generation_high), 20);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, code), 24);
        assert_eq!(std::mem::offset_of!(LinopRdpEvent, payload_len), 28);
        assert_eq!(
            std::mem::offset_of!(LinopRdpEventCallbackOptions, struct_size),
            0
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpEventCallbackOptions, abi_version),
            4
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpEventCallbackOptions, generation_low),
            8
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpEventCallbackOptions, generation_high),
            12
        );
    }

    #[test]
    fn credential_layout_matches_the_current_pointer_width() {
        assert_eq!(std::mem::offset_of!(LinopRdpBorrowedSecret, data), 0);
        assert_eq!(
            std::mem::offset_of!(LinopRdpCredentialBundle, struct_size),
            0
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpCredentialBundle, abi_version),
            4
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpCredentialBundle, server_password),
            8
        );

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<LinopRdpBorrowedSecret>(), 16);
            assert_eq!(align_of::<LinopRdpBorrowedSecret>(), 8);
            assert_eq!(std::mem::offset_of!(LinopRdpBorrowedSecret, len), 8);
            assert_eq!(size_of::<LinopRdpCredentialBundle>(), 112);
            assert_eq!(align_of::<LinopRdpCredentialBundle>(), 8);
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_password),
                24
            );
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, flags), 40);
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, username), 48);
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, domain), 64);
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username),
                80
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain),
                96
            );
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(size_of::<LinopRdpBorrowedSecret>(), 8);
            assert_eq!(align_of::<LinopRdpBorrowedSecret>(), 4);
            assert_eq!(std::mem::offset_of!(LinopRdpBorrowedSecret, len), 4);
            assert_eq!(size_of::<LinopRdpCredentialBundle>(), 60);
            assert_eq!(align_of::<LinopRdpCredentialBundle>(), 4);
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_password),
                16
            );
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, flags), 24);
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, username), 28);
            assert_eq!(std::mem::offset_of!(LinopRdpCredentialBundle, domain), 36);
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username),
                44
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain),
                52
            );
        }
    }

    #[test]
    fn connection_layout_matches_the_current_pointer_width() {
        assert_eq!(std::mem::offset_of!(LinopRdpBorrowedUtf16, data), 0);
        assert_eq!(
            std::mem::offset_of!(LinopRdpConnectionOptions, struct_size),
            0
        );
        assert_eq!(
            std::mem::offset_of!(LinopRdpConnectionOptions, abi_version),
            4
        );
        assert_eq!(std::mem::offset_of!(LinopRdpConnectionOptions, host), 8);

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<LinopRdpBorrowedUtf16>(), 16);
            assert_eq!(align_of::<LinopRdpBorrowedUtf16>(), 8);
            assert_eq!(std::mem::offset_of!(LinopRdpBorrowedUtf16, len), 8);
            assert_eq!(size_of::<LinopRdpConnectionOptions>(), 152);
            assert_eq!(align_of::<LinopRdpConnectionOptions>(), 8);
            assert_eq!(std::mem::offset_of!(LinopRdpConnectionOptions, port), 24);
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, desktop_width),
                28
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, desktop_height),
                32
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, color_depth),
                36
            );
            assert_eq!(std::mem::offset_of!(LinopRdpConnectionOptions, flags), 40);
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, display_mode),
                48
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, gateway_hostname),
                120
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, max_reconnect_attempts),
                148
            );
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(size_of::<LinopRdpBorrowedUtf16>(), 8);
            assert_eq!(align_of::<LinopRdpBorrowedUtf16>(), 4);
            assert_eq!(std::mem::offset_of!(LinopRdpBorrowedUtf16, len), 4);
            assert_eq!(size_of::<LinopRdpConnectionOptions>(), 136);
            assert_eq!(align_of::<LinopRdpConnectionOptions>(), 4);
            assert_eq!(std::mem::offset_of!(LinopRdpConnectionOptions, port), 16);
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, desktop_width),
                20
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, desktop_height),
                24
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, color_depth),
                28
            );
            assert_eq!(std::mem::offset_of!(LinopRdpConnectionOptions, flags), 32);
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, display_mode),
                40
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, gateway_hostname),
                112
            );
            assert_eq!(
                std::mem::offset_of!(LinopRdpConnectionOptions, max_reconnect_attempts),
                132
            );
        }
    }

    #[test]
    fn create_options_split_the_generation_without_abi_alignment_risk() {
        let options = LinopRdpCreateOptions::current(0x1122_3344_aabb_ccdd);

        assert_eq!(options.generation_low, 0xaabb_ccdd);
        assert_eq!(options.generation_high, 0x1122_3344);
    }

    #[test]
    fn callback_options_split_the_generation_without_abi_alignment_risk() {
        let options = LinopRdpEventCallbackOptions::current(0x1122_3344_aabb_ccdd);

        assert_eq!(options.generation_low, 0xaabb_ccdd);
        assert_eq!(options.generation_high, 0x1122_3344);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_probe_preserves_an_extended_caller_result_size() {
        let options = LinopRdpProbeOptions::current();
        let mut result = LinopRdpProbeResult::current();
        result.struct_size += 16;

        let native_result = unsafe { probe(&options, &mut result) };

        assert_eq!(native_result, RESULT_OK);
        assert_eq!(
            result.struct_size,
            size_of::<LinopRdpProbeResult>() as u32 + 16
        );
        assert_eq!(result.abi_version, ABI_VERSION);
        assert_eq!(result.reserved, 0);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_credentials_validate_size_before_version_and_borrowed_pointers() {
        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let mut credentials = LinopRdpCredentialBundle {
            struct_size: size_of::<LinopRdpCredentialBundle>() as u32,
            abi_version: ABI_VERSION,
            server_password: LinopRdpBorrowedSecret {
                data: std::ptr::null(),
                len: 0,
            },
            gateway_password: LinopRdpBorrowedSecret {
                data: std::ptr::null(),
                len: 0,
            },
            flags: 0,
            username: LinopRdpBorrowedUtf16 {
                data: std::ptr::null(),
                len: 0,
            },
            domain: LinopRdpBorrowedUtf16 {
                data: std::ptr::null(),
                len: 0,
            },
            gateway_username: LinopRdpBorrowedUtf16 {
                data: std::ptr::null(),
                len: 0,
            },
            gateway_domain: LinopRdpBorrowedUtf16 {
                data: std::ptr::null(),
                len: 0,
            },
        };

        credentials.struct_size = 4;
        credentials.abi_version += 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );

        credentials.struct_size = size_of::<LinopRdpCredentialBundle>() as u32;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_ABI_MISMATCH
        );

        credentials.abi_version = ABI_VERSION;
        credentials.server_password.len = 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );

        credentials.server_password.len = 0;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_UNAVAILABLE
        );

        credentials.username.len = 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );
        credentials.username.len = 0;
        credentials.domain.len = 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );
        credentials.domain.len = 0;

        credentials.struct_size = if cfg!(target_pointer_width = "64") {
            48
        } else {
            28
        };
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_UNAVAILABLE
        );
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_credentials_accept_append_only_callers_and_validate_gateway_identity() {
        #[repr(C)]
        struct ExtendedCredentials {
            base: LinopRdpCredentialBundle,
            trailing: [u8; 16],
        }

        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let empty_secret = LinopRdpBorrowedSecret {
            data: std::ptr::null(),
            len: 0,
        };
        let empty_text = LinopRdpBorrowedUtf16 {
            data: std::ptr::null(),
            len: 0,
        };
        let mut credentials = LinopRdpCredentialBundle {
            struct_size: size_of::<LinopRdpCredentialBundle>() as u32,
            abi_version: ABI_VERSION,
            server_password: empty_secret,
            gateway_password: empty_secret,
            flags: 0,
            username: empty_text,
            domain: empty_text,
            gateway_username: empty_text,
            gateway_domain: empty_text,
        };

        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_UNAVAILABLE
        );

        credentials.gateway_username.len = 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );
        credentials.gateway_username.len = 0;
        credentials.gateway_domain.len = 1;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_INVALID_ARGUMENT
        );
        credentials.gateway_domain.len = 0;

        credentials.gateway_username.len = 1;
        credentials.struct_size = (std::mem::offset_of!(LinopRdpCredentialBundle, gateway_username)
            + size_of::<LinopRdpBorrowedUtf16>()
            - 1) as u32;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_UNAVAILABLE
        );
        credentials.gateway_username.len = 0;

        credentials.gateway_domain.len = 1;
        credentials.struct_size = (std::mem::offset_of!(LinopRdpCredentialBundle, gateway_domain)
            + size_of::<LinopRdpBorrowedUtf16>()
            - 1) as u32;
        assert_eq!(
            unsafe { apply_credentials(host, &credentials) },
            RESULT_UNAVAILABLE
        );

        let trailing = [0xA5; 16];
        let extended = ExtendedCredentials {
            base: LinopRdpCredentialBundle {
                struct_size: size_of::<ExtendedCredentials>() as u32,
                abi_version: ABI_VERSION,
                server_password: empty_secret,
                gateway_password: empty_secret,
                flags: 0,
                username: empty_text,
                domain: empty_text,
                gateway_username: empty_text,
                gateway_domain: empty_text,
            },
            trailing,
        };
        assert_eq!(
            unsafe { apply_credentials(host, &extended.base) },
            RESULT_UNAVAILABLE
        );
        assert_eq!(extended.trailing, trailing);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_connection_validates_layout_values_and_outputs_before_unavailable() {
        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let host_name: Vec<u16> = "rdp.example".encode_utf16().collect();
        let mut options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );

        options.struct_size = 4;
        options.abi_version += 1;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.struct_size = size_of::<LinopRdpConnectionOptions>() as u32;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_ABI_MISMATCH);

        options.abi_version = ABI_VERSION;
        options.host.data = std::ptr::null();
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.host.data = host_name.as_ptr();
        options.port = 0;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.port = 3389;
        options.flags = CONNECTION_FLAG_AUDIO_PLAYBACK_DISABLED;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        options.flags = CONNECTION_FLAGS_KNOWN << 1;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.flags = 0;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        let mut state = u32::MAX;
        assert_eq!(
            unsafe { get_connection_state(std::ptr::null_mut(), &mut state) },
            RESULT_INVALID_ARGUMENT
        );
        assert_eq!(state, CONNECTION_STATE_DISCONNECTED);

        let mut status = u32::MAX;
        assert_eq!(
            unsafe { request_close(std::ptr::null_mut(), &mut status) },
            RESULT_INVALID_ARGUMENT
        );
        assert_eq!(status, REQUEST_CLOSE_CAN_PROCEED);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_connection_accepts_legacy_current_and_larger_callers() {
        #[repr(C)]
        struct ExtendedConnectionOptions {
            base: LinopRdpConnectionOptions,
            trailing: [u8; 16],
        }

        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let host_name: Vec<u16> = "rdp.example".encode_utf16().collect();
        let mut options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );

        options.struct_size = if cfg!(target_pointer_width = "64") {
            48
        } else {
            36
        };
        options.legacy_reserved = u32::MAX;
        options.display_mode = u32::MAX;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        options.struct_size -= 1;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );
        options.legacy_reserved = u32::MAX;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        let trailing = [0x5A; 16];
        let extended = ExtendedConnectionOptions {
            base: LinopRdpConnectionOptions {
                struct_size: size_of::<ExtendedConnectionOptions>() as u32,
                ..options
            },
            trailing,
        };
        assert_eq!(unsafe { connect(host, &extended.base) }, RESULT_UNAVAILABLE);
        assert_eq!(extended.trailing, trailing);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_connection_ignores_partial_append_fields_and_preserves_legacy_audio() {
        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let host_name: Vec<u16> = "rdp.example".encode_utf16().collect();
        let mut options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );

        options.display_mode = u32::MAX;
        options.struct_size = (std::mem::offset_of!(LinopRdpConnectionOptions, display_mode)
            + size_of::<u32>()
            - 1) as u32;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );
        options.gateway_hostname = LinopRdpBorrowedUtf16 {
            data: std::ptr::null(),
            len: 1,
        };
        options.struct_size = (std::mem::offset_of!(LinopRdpConnectionOptions, gateway_hostname)
            + size_of::<LinopRdpBorrowedUtf16>()
            - 1) as u32;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);

        options.struct_size = if cfg!(target_pointer_width = "64") {
            48
        } else {
            36
        };
        options.flags = CONNECTION_FLAG_AUDIO_PLAYBACK_DISABLED;
        options.audio_mode = u32::MAX;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);
    }

    #[cfg(not(windows_rdp_host_native))]
    #[test]
    fn non_windows_connection_validates_complete_policy_tail() {
        let host = std::ptr::NonNull::<NativeRdpHost>::dangling().as_ptr();
        let host_name: Vec<u16> = "rdp.example".encode_utf16().collect();
        let gateway_name: Vec<u16> = "gateway.example".encode_utf16().collect();
        let mut options = LinopRdpConnectionOptions::current(
            LinopRdpBorrowedUtf16 {
                data: host_name.as_ptr(),
                len: host_name.len() as u32,
            },
            3389,
            1920,
            1080,
            32,
        );

        macro_rules! invalid_field {
            ($field:ident, $value:expr) => {{
                let previous = options.$field;
                options.$field = $value;
                assert_eq!(
                    unsafe { connect(host, &options) },
                    RESULT_INVALID_ARGUMENT,
                    stringify!($field)
                );
                options.$field = previous;
            }};
        }

        invalid_field!(display_mode, 2);
        invalid_field!(display_flags, DISPLAY_FLAGS_KNOWN << 1);
        invalid_field!(desktop_scale_factor, 99);
        invalid_field!(desktop_scale_factor, 501);
        invalid_field!(device_scale_factor, 101);
        invalid_field!(resource_flags, RESOURCE_FLAGS_KNOWN << 1);
        invalid_field!(audio_mode, 3);
        invalid_field!(audio_quality, 3);
        invalid_field!(audio_flags, AUDIO_FLAGS_KNOWN << 1);
        invalid_field!(keyboard_hook_mode, 3);
        invalid_field!(input_flags, INPUT_FLAGS_KNOWN << 1);
        invalid_field!(performance_preset, 5);
        invalid_field!(performance_flags, PERFORMANCE_FLAGS_KNOWN << 1);
        invalid_field!(network_connection_type, 7);
        invalid_field!(security_flags, SECURITY_FLAGS_KNOWN << 1);
        invalid_field!(authentication_level, 3);
        invalid_field!(gateway_mode, 3);
        invalid_field!(gateway_flags, GATEWAY_FLAGS_KNOWN << 1);
        invalid_field!(gateway_credential_source, 2);
        invalid_field!(keep_alive_seconds, u32::MAX);
        invalid_field!(timeout_seconds, i32::MAX as u32 + 1);
        invalid_field!(connection_flags, CONNECTION_POLICY_FLAGS_KNOWN << 1);
        invalid_field!(max_reconnect_attempts, i32::MAX as u32 + 1);

        options.gateway_mode = 1;
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.gateway_hostname = LinopRdpBorrowedUtf16 {
            data: std::ptr::null(),
            len: 1,
        };
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        let gateway_with_nul: Vec<u16> = "gateway\0example".encode_utf16().collect();
        options.gateway_hostname = LinopRdpBorrowedUtf16 {
            data: gateway_with_nul.as_ptr(),
            len: gateway_with_nul.len() as u32,
        };
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        let oversized_gateway =
            vec![b'g' as u16; crate::policy::WINDOWS_RDP_MAX_GATEWAY_HOST_UTF16_CODE_UNITS + 1];
        options.gateway_hostname = LinopRdpBorrowedUtf16 {
            data: oversized_gateway.as_ptr(),
            len: oversized_gateway.len() as u32,
        };
        assert_eq!(unsafe { connect(host, &options) }, RESULT_INVALID_ARGUMENT);

        options.gateway_hostname = LinopRdpBorrowedUtf16 {
            data: gateway_name.as_ptr(),
            len: gateway_name.len() as u32,
        };
        assert_eq!(unsafe { connect(host, &options) }, RESULT_UNAVAILABLE);
    }
}
