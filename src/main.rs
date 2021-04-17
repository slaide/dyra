#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(target_os="windows")]
use winapi::{
    um::{
        libloaderapi::{
            GetModuleHandleA
        },
        winnt::{
            LPCSTR,
        },
        winuser::{
            WNDCLASSA,
            DefWindowProcA,
            RegisterClassA,
            MSG,
            WM_CLOSE,
            PeekMessageA,
            PM_REMOVE,
            TranslateMessage,
            DispatchMessageA,
            CreateWindowExA,
            WS_OVERLAPPEDWINDOW,
            DestroyWindow,
            ShowWindow,
            SW_SHOWDEFAULT,
            UpdateWindow,
        }
    },
    shared::{
        windef::{
            HWND
        },
        minwindef::{
            HINSTANCE,
            LPARAM,
            WPARAM,
            LRESULT,
        }
    }
};


#[cfg(target_os="linux")]
extern crate xcb;

#[cfg(target_os="linux")]
use xcb::{

};

#[derive(PartialEq,Debug)]
enum ControlFlow{
    Continue,
    Stop,
}
#[derive(PartialEq,Debug)]
enum Event{
    WindowCloseRequested,
}

enum WindowHandle{
    #[cfg(target_os="windows")]
    Windows{
        hwnd:HWND,
    }
}
impl WindowHandle{
}
struct Window{
    handle:WindowHandle,
}
impl Window{
}
impl Drop for Window{
    fn drop(&mut self){
        #[allow(unreachable_patterns)]
        match self.handle{
            WindowHandle::Windows{hwnd}=>{
                unsafe{
                    DestroyWindow(hwnd);
                }
            },
            _=>unreachable!()
        }
    }
}

#[cfg(target_os="windows")]
static mut WINDOWS_WINDOW_EVENTS:Vec<Event>=Vec::new();
#[cfg(target_os="windows")]
unsafe extern "system" fn windowproc(window:HWND,umsg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    match umsg{
        WM_CLOSE=>{
            WINDOWS_WINDOW_EVENTS.push(Event::WindowCloseRequested);
            return 0;
        },
        _=>DefWindowProcA(window,umsg,wparam,lparam)
    }
}

enum WindowManagerHandle{
    #[cfg(target_os="windows")]
    Windows{
        hinstance:HINSTANCE,
        class_name:String,
    },
}
impl WindowManagerHandle{
    pub fn new()->Self{
        #[cfg(target_os="windows")]
        {
            let hinstance=unsafe{
                GetModuleHandleA(std::ptr::null())
            } as HINSTANCE;
            if hinstance==std::ptr::null_mut(){
                panic!("hInstance")
            }
            let mut class:WNDCLASSA=unsafe{
                std::mem::zeroed()
            };
            class.lpfnWndProc=Some(windowproc);
            class.hInstance=hinstance;
            let class_name=String::from("mywindowclass");
            class.lpszClassName=class_name.as_str().as_ptr() as LPCSTR;//needs to be same address as the one used for CreateWindowEx
            unsafe{
                RegisterClassA(&class)
            };

            Self::Windows{
                hinstance,
                class_name
            }
        }
        #[cfg(not(target_os="windows"))]
        {
            unimplemented!()
        }
    }
}
struct WindowManager{
    handle:WindowManagerHandle,
    open_windows:Vec<Window>
}
impl WindowManager{
    pub fn new()->Self{
        Self{
            handle:WindowManagerHandle::new(),
            open_windows:Vec::new()
        }
    }
    pub fn new_window(&mut self,width:u16,height:u16){
        let window=Window{
            handle:{
                #[allow(unreachable_patterns)]
                match &self.handle{
                    WindowManagerHandle::Windows{hinstance,class_name}=>{
                        let window_hwnd:HWND=unsafe{
                            CreateWindowExA(
                                0,
                                class_name.as_str().as_ptr() as LPCSTR,
                                "my window".as_ptr() as LPCSTR,
                                WS_OVERLAPPEDWINDOW,
                                0,
                                0,
                                width as i32,
                                height as i32,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                *hinstance,
                                std::ptr::null_mut(),
                            )
                        };
                        if window_hwnd==std::ptr::null_mut(){
                            panic!("CreateWindowExA")
                        }

                        unsafe{
                            ShowWindow(window_hwnd,SW_SHOWDEFAULT);
                            UpdateWindow(window_hwnd);
                        }

                        WindowHandle::Windows{
                            hwnd:window_hwnd,
                        }
                    },
                    _=>unimplemented!()
                }
            },
        };
        self.open_windows.push(window);
    }
    pub fn step(&mut self)->ControlFlow{
        #[allow(unreachable_patterns)]
        match self.handle{
            WindowManagerHandle::Windows{..}=>{
                let mut msg:MSG=unsafe{
                    std::mem::zeroed()
                };
                while unsafe{
                    PeekMessageA(&mut msg,std::ptr::null_mut(),0,0,PM_REMOVE)>0
                }{
                    unsafe{
                        TranslateMessage(&mut msg);
                        DispatchMessageA(&mut msg);
                    }
                }
        
                #[cfg(target_os="windows")]
                {
                    for ev in unsafe{
                        WINDOWS_WINDOW_EVENTS.iter()
                    }{
                        match ev{
                            Event::WindowCloseRequested=>{
                                return ControlFlow::Stop;
                            }
                            _=>{}
                        }
                    }
                    unsafe{
                        WINDOWS_WINDOW_EVENTS.clear();
                    }
                }
            },
            _=>panic!("unsupported")
        }
        ControlFlow::Continue
    }
}

struct Manager{
    window_manager: WindowManager,
}
impl Manager{
    pub fn new()->Self{
        Self{
            window_manager:WindowManager::new()
        }
    }
    pub fn step(&mut self)->ControlFlow{
        self.window_manager.step()
    }
    pub fn run(&mut self){
        loop{
            if self.step()!=ControlFlow::Continue{
                break;
            }

            //cap framerate at 30fps
            std::thread::sleep(std::time::Duration::from_millis(1000/30));

            println!("step done");
        }
    }
}

fn main() {
    let mut manager=Manager::new();
    manager.window_manager.new_window(600,400);
    manager.run();
}
