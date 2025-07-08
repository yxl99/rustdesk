
use hbb_common::{anyhow, dlopen::symbor::Library, log, ResultType};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

const LIB_NAME_VIRTUAL_DISPLAY: &str = "dylib_virtual_display";
const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;
const DEFAULT_REFRESH_RATE: u32 = 60;

pub type DWORD = ::std::os::raw::c_ulong;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _MonitorMode {
    pub width: DWORD,
    pub height: DWORD,
    pub sync: DWORD,
}
pub type MonitorMode = _MonitorMode;
pub type PMonitorMode = *mut MonitorMode;

// 保留原有函数指针类型定义
pub type GetDriverInstallPath = fn() -> &'static str;
pub type IsDeviceCreated = fn() -> bool;
pub type CloseDevice = fn();
pub type CreateDevice = fn() -> ResultType<()>;
pub type PlugInMonitor = fn(u32, u32, u32) -> ResultType<()>;
pub type PlugOutMonitor = fn(u32) -> ResultType<()>;
pub type UpdateMonitorModes = fn(u32, u32, PMonitorMode) -> ResultType<()>;

lazy_static! {
    static ref LIB_WRAPPER: Arc<Mutex<LibWrapper>> = Arc::new(Mutex::new(LibWrapper::new()));
    static ref MONITOR_INDICES: Mutex<HashSet<u32>> = Mutex::new(HashSet::new());
}

struct LibWrapper {
    _lib: Option<Library>,
    get_driver_install_path: Option<GetDriverInstallPath>,
    is_device_created: Option<IsDeviceCreated>,
    close_device: Option<CloseDevice>,
    create_device: Option<CreateDevice>,
    plug_in_monitor: Option<PlugInMonitor>,
    plug_out_monitor: Option<PlugOutMonitor>,
    update_monitor_modes: Option<UpdateMonitorModes>,
}

impl LibWrapper {
    fn new() -> Self {
        let lib = Library::open(get_lib_name()).ok();
        Self {
            _lib: lib,
            get_driver_install_path: None,
            is_device_created: None,
            close_device: None,
            create_device: None,
            plug_in_monitor: None,
            plug_out_monitor: None,
            update_monitor_modes: None,
        }
    }

    pub fn ensure_display(&self) -> ResultType<()> {
        if let Some(create) = self.create_device {
            create()?;
            if MONITOR_INDICES.lock().unwrap().is_empty() {
                self.add_default_monitor()?;
            }
            Ok(())
        } else {
            Err(anyhow!("CreateDevice function not available"))
        }
    }

    fn add_default_monitor(&self) -> ResultType<()> {
        let modes = [
            MonitorMode { width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT, sync: DEFAULT_REFRESH_RATE },
            MonitorMode { width: 1280, height: 720, sync: 60 },
        ];
        
        let idx = generate_monitor_index();
        if let Some(plug_in) = self.plug_in_monitor {
            plug_in(idx, DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_REFRESH_RATE)?;
            if let Some(update) = self.update_monitor_modes {
                update(idx, modes.len() as u32, &modes as *const _ as PMonitorMode)?;
            }
            MONITOR_INDICES.lock().unwrap().insert(idx);
        }
        Ok(())
    }
}

fn generate_monitor_index() -> u32 {
    let mut indices = MONITOR_INDICES.lock().unwrap();
    let mut candidate = 0;
    while indices.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

fn get_lib_name() -> String {
    #[cfg(windows)] { format!("{}.dll", LIB_NAME_VIRTUAL_DISPLAY) }
    #[cfg(unix)] { format!("lib{}.so", LIB_NAME_VIRTUAL_DISPLAY) }
    #[cfg(target_os = "macos")] { format!("lib{}.dylib", LIB_NAME_VIRTUAL_DISPLAY) }
}
