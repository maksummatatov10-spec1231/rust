//! Forest Bloom 3D — native Windows build.
//!
//! Same Rust gameplay core as the WASM version, rendered with raw Win32 + OpenGL
//! (OpenGL 1.1 compatibility profile, no external crates and no browser).

#[cfg(not(windows))]
fn main() {
    eprintln!("This binary is the native Windows build. Compile it with: cargo build --release --target x86_64-pc-windows-gnu");
}

#[cfg(windows)]
mod app {
    use core::ffi::c_void;
    use core::mem;
    use core::ptr;

    #[path = "game.rs"]
    pub mod game;

    const WM_CLOSE: u32 = 0x0010;
    const WM_DESTROY: u32 = 0x0002;
    const WM_QUIT: u32 = 0x0012;
    const PM_REMOVE: u32 = 0x0001;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
    const CW_USEDEFAULT: i32 = -2147483648;
    const SW_SHOW: i32 = 5;
    const PFD_SUPPORT_OPENGL: u32 = 0x00000020;
    const PFD_DRAW_TO_WINDOW: u32 = 0x00000004;
    const PFD_DOUBLEBUFFER: u32 = 0x00000001;
    const PFD_TYPE_RGBA: u8 = 0;
    const PFD_MAIN_PLANE: u8 = 0;
    const GL_COLOR_BUFFER_BIT: u32 = 0x00004000;
    const GL_DEPTH_BUFFER_BIT: u32 = 0x00000100;
    const GL_DEPTH_TEST: u32 = 0x0B71;
    const GL_PROJECTION: u32 = 0x1701;
    const GL_MODELVIEW: u32 = 0x1700;
    const GL_LINES: u32 = 0x0001;
    const GL_QUADS: u32 = 0x0007;
    const GL_TRIANGLES: u32 = 0x0004;
    const GL_TRIANGLE_FAN: u32 = 0x0006;
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    const GL_LIST_BASE: u32 = 0x0B32;

    type Hwnd = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point { x: i32, y: i32 }

    #[repr(C)]
    struct Rect { left: i32, top: i32, right: i32, bottom: i32 }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd, message: u32, wparam: usize, lparam: isize,
        time: u32, pt: Point,
    }

    #[repr(C)]
    struct WndClassW {
        style: u32,
        lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize>,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: *mut c_void,
        h_icon: *mut c_void,
        h_cursor: *mut c_void,
        hbr_background: *mut c_void,
        lpsz_menu_name: *const u16,
        lpsz_class_name: *const u16,
    }

    #[repr(C)]
    struct PixelFormatDescriptor {
        n_size: u16, n_version: u16, dw_flags: u32,
        i_pixel_type: u8, c_color_bits: u8,
        c_red_bits: u8, c_red_shift: u8, c_green_bits: u8, c_green_shift: u8,
        c_blue_bits: u8, c_blue_shift: u8,
        c_alpha_bits: u8, c_alpha_shift: u8,
        c_accum_bits: u8, c_accum_red_bits: u8, c_accum_green_bits: u8,
        c_accum_blue_bits: u8, c_accum_alpha_bits: u8,
        c_depth_bits: u8, c_stencil_bits: u8, c_aux_buffers: u8, i_layer_type: u8,
        dw_visible_mask: u32, dw_damage_mask: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn QueryPerformanceCounter(v: *mut i64) -> i32;
        fn QueryPerformanceFrequency(v: *mut i64) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(c: *const WndClassW) -> u16;
        fn CreateWindowExW(style: u32, class: *const u16, title: *const u16,
                           dw_style: u32, x: i32, y: i32, w: i32, h: i32,
                           parent: Hwnd, menu: *mut c_void, inst: *mut c_void,
                           param: *mut c_void) -> Hwnd;
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn UpdateWindow(h: Hwnd) -> i32;
        fn PeekMessageW(msg: *mut Msg, h: Hwnd, filter_min: u32, filter_max: u32, remove: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn DefWindowProcW(h: Hwnd, msg: u32, w: usize, l: isize) -> isize;
        fn PostQuitMessage(code: i32);
        fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
        fn GetDC(h: Hwnd) -> *mut c_void;
        fn ReleaseDC(h: Hwnd, dc: *mut c_void) -> i32;
        fn GetAsyncKeyState(key: i32) -> i16;
        fn GetCursorPos(p: *mut Point) -> i32;
        fn SetCursorPos(x: i32, y: i32) -> i32;
        fn ShowCursor(show: i32) -> i32;
        fn MessageBoxW(h: Hwnd, text: *const u16, caption: *const u16, typ: u32) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn ChoosePixelFormat(dc: *mut c_void, pfd: *const PixelFormatDescriptor) -> i32;
        fn SetPixelFormat(dc: *mut c_void, fmt: i32, pfd: *const PixelFormatDescriptor) -> i32;
        fn SwapBuffers(dc: *mut c_void) -> i32;
        fn CreateFontW(h: i32, w: i32, esc: i32, orient: i32, weight: i32,
                       italic: i32, underline: i32, strike: i32, charset: u32,
                       out_prec: u32, clip_prec: u32, quality: u32, pitch: u32,
                       face: *const u16) -> *mut c_void;
        fn SelectObject(dc: *mut c_void, obj: *mut c_void) -> *mut c_void;
    }

    #[link(name = "opengl32")]
    extern "system" {
        fn wglCreateContext(dc: *mut c_void) -> *mut c_void;
        fn wglMakeCurrent(dc: *mut c_void, ctx: *mut c_void) -> i32;
        fn wglUseFontBitmapsW(dc: *mut c_void, first: u32, count: u32, base: u32) -> i32;
        fn glClearColor(r: f32, g: f32, b: f32, a: f32);
        fn glClear(mask: u32);
        fn glViewport(x: i32, y: i32, w: i32, h: i32);
        fn glMatrixMode(mode: u32);
        fn glLoadIdentity();
        fn glPushMatrix();
        fn glPopMatrix();
        fn glFrustum(l: f64, r: f64, b: f64, t: f64, n: f64, f: f64);
        fn glOrtho(l: f64, r: f64, b: f64, t: f64, n: f64, f: f64);
        fn glTranslatef(x: f32, y: f32, z: f32);
        fn glRotatef(a: f32, x: f32, y: f32, z: f32);
        fn glScalef(x: f32, y: f32, z: f32);
        fn glEnable(cap: u32);
        fn glDisable(cap: u32);
        fn glBegin(mode: u32);
        fn glEnd();
        fn glColor3f(r: f32, g: f32, b: f32);
        fn glVertex2f(x: f32, y: f32);
        fn glVertex3f(x: f32, y: f32, z: f32);
        fn glRasterPos2f(x: f32, y: f32);
        fn glListBase(base: u32);
        fn glCallLists(n: i32, typ: u32, lists: *const c_void);
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(core::iter::once(0)).collect()
    }

    unsafe extern "system" fn wnd_proc(h: Hwnd, msg: u32, w: usize, l: isize) -> isize {
        match msg {
            WM_CLOSE | WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcW(h, msg, w, l),
        }
    }

    fn seeded(n: u32) -> f32 {
        let mut x = n.wrapping_mul(374761393).wrapping_add(668265263);
        x = (x ^ (x >> 13)).wrapping_mul(1274126177);
        ((x ^ (x >> 16)) as f32) / 4294967296.0
    }

    unsafe fn text(x: f32, y: f32, s: &str) {
        let w = wide(s);
        glRasterPos2f(x, y);
        glCallLists((w.len() - 1) as i32, GL_UNSIGNED_BYTE, w.as_ptr() as *const c_void);
    }

    unsafe fn box_at(cx: f32, cy: f32, cz: f32, sx: f32, sy: f32, sz: f32,
                     r: f32, g: f32, b: f32) {
        glColor3f(r, g, b);
        glBegin(GL_QUADS);
        let x = sx / 2.0; let y = sy / 2.0; let z = sz / 2.0;
        // front / back
        glVertex3f(cx - x, cy - y, cz + z); glVertex3f(cx + x, cy - y, cz + z);
        glVertex3f(cx + x, cy + y, cz + z); glVertex3f(cx - x, cy + y, cz + z);
        glVertex3f(cx + x, cy - y, cz - z); glVertex3f(cx - x, cy - y, cz - z);
        glVertex3f(cx - x, cy + y, cz - z); glVertex3f(cx + x, cy + y, cz - z);
        // sides
        glVertex3f(cx - x, cy - y, cz - z); glVertex3f(cx - x, cy - y, cz + z);
        glVertex3f(cx - x, cy + y, cz + z); glVertex3f(cx - x, cy + y, cz - z);
        glVertex3f(cx + x, cy - y, cz + z); glVertex3f(cx + x, cy - y, cz - z);
        glVertex3f(cx + x, cy + y, cz - z); glVertex3f(cx + x, cy + y, cz + z);
        // top / bottom
        glVertex3f(cx - x, cy + y, cz + z); glVertex3f(cx + x, cy + y, cz + z);
        glVertex3f(cx + x, cy + y, cz - z); glVertex3f(cx - x, cy + y, cz - z);
        glVertex3f(cx - x, cy - y, cz - z); glVertex3f(cx + x, cy - y, cz - z);
        glVertex3f(cx + x, cy - y, cz + z); glVertex3f(cx - x, cy - y, cz + z);
        glEnd();
    }

    unsafe fn tree(x: f32, z: f32, scale: f32) {
        glPushMatrix();
        glTranslatef(x, 0.0, z);
        glScalef(scale, scale, scale);
        box_at(0.0, 1.4, 0.0, 0.34, 2.8, 0.34, 0.32, 0.20, 0.11);
        glColor3f(0.14, 0.42 + seeded(x as u32) * 0.12, 0.19);
        glBegin(GL_TRIANGLES);
        for i in 0..7 {
            let a0 = (i as f32) * 0.897;
            let a1 = ((i + 1) as f32) * 0.897;
            glVertex3f(a0.sin() * 1.7, 2.3, a0.cos() * 1.7);
            glVertex3f(a1.sin() * 1.7, 2.3, a1.cos() * 1.7);
            glVertex3f(0.0, 6.2, 0.0);
        }
        glEnd();
        glPopMatrix();
    }

    unsafe fn draw_world(px: f32, pz: f32) {
        // Sun.
        glColor3f(1.0, 0.92, 0.66);
        glBegin(GL_TRIANGLE_FAN);
        glVertex3f(px + 60.0, 75.0, pz - 95.0);
        for i in 0..20 {
            let a = (i as f32) * 0.314159;
            glVertex3f(px + 60.0 + a.cos() * 9.0, 75.0 + a.sin() * 9.0, pz - 95.0);
        }
        glEnd();

        // Ground grid.
        glColor3f(0.14, 0.29, 0.17);
        glBegin(GL_LINES);
        for i in -12..=12 {
            let v = i as f32 * 4.0;
            glVertex3f(-48.0, 0.0, v); glVertex3f(48.0, 0.0, v);
            glVertex3f(v, 0.0, -48.0); glVertex3f(v, 0.0, 48.0);
        }
        glEnd();

        // Forest.
        for i in 0..90u32 {
            let a = seeded(i * 3 + 1) * 6.2831853;
            let r = 9.0 + seeded(i * 3 + 2).sqrt() * 37.0;
            let x = a.cos() * r;
            let z = a.sin() * r;
            tree(x, z, 0.75 + seeded(i * 9) * 0.7);
        }
        for i in 0..50u32 {
            let a = seeded(i * 7 + 61) * 6.2831853;
            let r = 4.0 + seeded(i * 7 + 62).sqrt() * 38.0;
            let s = 0.25 + seeded(i * 11) * 0.35;
            box_at(a.cos() * r, s * 0.5, a.sin() * r, s, s, s, 0.30, 0.36, 0.24);
        }

        // Enemies and pickups from the Rust core.
        let base = game::foes_ptr();
        for i in 0..game::foes_count() as usize {
            let f = &*(base.add(i) as *const game::Foe);
            if f.kind == 3 {
                box_at(f.x, 0.65, f.z, 0.5, 0.5, 0.5, 0.35, 1.0, 0.45);
            } else {
                let h = if f.kind == 2 { 2.4 } else { 1.8 };
                let w = if f.kind == 2 { 1.0 } else { 0.72 };
                box_at(f.x, h / 2.0, f.z, w, h, w, 0.28, 0.14, 0.36);
                box_at(f.x - 0.18, h * 0.78, f.z + w / 2.0 + 0.02, 0.12, 0.12, 0.04, 1.0, 0.25, 0.8);
                box_at(f.x + 0.18, h * 0.78, f.z + w / 2.0 + 0.02, 0.12, 0.12, 0.04, 1.0, 0.25, 0.8);
            }
        }
    }

    unsafe fn draw_sword(attack: f32) {
        glMatrixMode(GL_MODELVIEW);
        glLoadIdentity();
        let swing = (attack * core::f32::consts::PI).sin();
        glTranslatef(0.42 - swing * 0.18, -0.36, -0.75);
        glRotatef(-18.0 + swing * 75.0, 0.0, 0.0, 1.0);
        glRotatef(-12.0 + swing * 25.0, 1.0, 0.0, 0.0);
        box_at(0.0, 0.0, 0.0, 0.12, 0.30, 0.10, 0.42, 0.24, 0.10); // hilt
        box_at(0.0, 0.18, 0.0, 0.38, 0.07, 0.10, 0.75, 0.62, 0.22); // guard
        box_at(0.0, 0.78, 0.0, 0.11, 1.10, 0.055, 0.82, 0.88, 0.94); // blade
    }

    unsafe fn draw_hud(w: i32, h: i32) {
        glMatrixMode(GL_PROJECTION);
        glPushMatrix();
        glLoadIdentity();
        glOrtho(0.0, w as f64, 0.0, h as f64, -1.0, 1.0);
        glMatrixMode(GL_MODELVIEW);
        glPushMatrix();
        glLoadIdentity();
        glDisable(GL_DEPTH_TEST);

        let hp = game::state(0).max(0.0) / 100.0;
        let st = game::state(1).max(0.0) / 100.0;
        box_at(22.0, h as f32 - 34.0, 0.0, 220.0, 12.0, 0.0, 0.10, 0.04, 0.05);
        box_at(22.0 + 110.0 * hp, h as f32 - 34.0, 0.0, 220.0 * hp, 12.0, 0.0, 0.92, 0.30, 0.32);
        box_at(22.0, h as f32 - 54.0, 0.0, 220.0, 8.0, 0.0, 0.04, 0.10, 0.08);
        box_at(22.0 + 110.0 * st, h as f32 - 54.0, 0.0, 220.0 * st, 8.0, 0.0, 0.32, 0.85, 0.72);

        // Crosshair.
        glColor3f(0.9, 1.0, 0.9);
        glBegin(GL_LINES);
        glVertex2f((w / 2 - 9) as f32, (h / 2) as f32); glVertex2f((w / 2 + 9) as f32, (h / 2) as f32);
        glVertex2f((w / 2) as f32, (h / 2 - 9) as f32); glVertex2f((w / 2) as f32, (h / 2 + 9) as f32);
        glEnd();

        text(22.0, h as f32 - 72.0, "HP / STAMINA");
        text(w as f32 - 190.0, h as f32 - 34.0, "FOREST BLOOM 3D");
        text(w as f32 - 190.0, h as f32 - 56.0, "WASD  MOUSE  LMB  ESC");

        if game::mode() == 0 {
            box_at(w as f32 / 2.0, h as f32 / 2.0, 0.0, 520.0, 230.0, 0.0, 0.03, 0.09, 0.06);
            text(w as f32 / 2.0 - 180.0, h as f32 / 2.0 + 60.0, "FOREST BLOOM 3D");
            text(w as f32 / 2.0 - 180.0, h as f32 / 2.0 + 20.0, "PRESS ENTER TO START");
            text(w as f32 / 2.0 - 180.0, h as f32 / 2.0 - 20.0, "WASD move  mouse look  LMB sword");
            text(w as f32 / 2.0 - 180.0, h as f32 / 2.0 - 60.0, "SHIFT sprint  SPACE jump  ESC pause");
        } else if game::mode() == 2 {
            text(w as f32 / 2.0 - 70.0, h as f32 / 2.0, "PAUSED - ENTER");
        } else if game::mode() == 3 {
            text(w as f32 / 2.0 - 150.0, h as f32 / 2.0, "GAME OVER - ENTER");
        }

        glEnable(GL_DEPTH_TEST);
        glMatrixMode(GL_PROJECTION);
        glPopMatrix();
        glMatrixMode(GL_MODELVIEW);
        glPopMatrix();
    }

    pub fn run() {
        unsafe {
            let class_name = wide("ForestBloomNative");
            let title = wide("Forest Bloom 3D - Rust native");
            let inst = GetModuleHandleW(ptr::null());
            let wc = WndClassW {
                style: 0x0002 | 0x0001, // CS_HREDRAW | CS_VREDRAW
                lpfn_wnd_proc: Some(wnd_proc),
                cb_cls_extra: 0, cb_wnd_extra: 0,
                h_instance: inst, h_icon: ptr::null_mut(),
                h_cursor: ptr::null_mut(), hbr_background: ptr::null_mut(),
                lpsz_menu_name: ptr::null(), lpsz_class_name: class_name.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                MessageBoxW(ptr::null_mut(), wide("RegisterClass failed").as_ptr(),
                            wide("Forest Bloom 3D").as_ptr(), 0);
                return;
            }
            let hwnd = CreateWindowExW(0, class_name.as_ptr(), title.as_ptr(),
                WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 1100, 700,
                ptr::null_mut(), ptr::null_mut(), inst, ptr::null_mut());
            if hwnd.is_null() { return; }
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            let dc = GetDC(hwnd);
            let mut pfd: PixelFormatDescriptor = mem::zeroed();
            pfd.n_size = mem::size_of::<PixelFormatDescriptor>() as u16;
            pfd.n_version = 1;
            pfd.dw_flags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
            pfd.i_pixel_type = PFD_TYPE_RGBA;
            pfd.c_color_bits = 24;
            pfd.c_depth_bits = 24;
            pfd.i_layer_type = PFD_MAIN_PLANE;
            let fmt = ChoosePixelFormat(dc, &pfd);
            if fmt == 0 || SetPixelFormat(dc, fmt, &pfd) == 0 { return; }
            let ctx = wglCreateContext(dc);
            if ctx.is_null() || wglMakeCurrent(dc, ctx) == 0 { return; }

            let font = CreateFontW(22, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 0, 0, ptr::null());
            if !font.is_null() {
                SelectObject(dc, font);
                wglUseFontBitmapsW(dc, 0, 256, 1000);
                glListBase(1000);
            }

            glEnable(GL_DEPTH_TEST);
            glClearColor(0.45, 0.68, 0.86, 1.0);
            game::init_game();

            let mut freq: i64 = 0;
            QueryPerformanceFrequency(&mut freq);
            let mut last: i64 = 0;
            QueryPerformanceCounter(&mut last);
            let mut msg: Msg = mem::zeroed();
            let mut enter_was_down = false;
            let mut running = true;

            while running {
                while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT { running = false; break; }
                    TranslateMessage(&mut msg);
                    DispatchMessageW(&mut msg);
                }
                if !running { break; }

                let mut now: i64 = 0;
                QueryPerformanceCounter(&mut now);
                let dt = ((now - last) as f32 / freq.max(1) as f32).min(0.05);
                last = now;

                let enter = (GetAsyncKeyState(0x0D) as u16 & 0x8000) != 0;
                if enter && !enter_was_down {
                    match game::mode() {
                        0 | 3 => { game::reset_game(); ShowCursor(0); }
                        2 => { game::set_mode(1); ShowCursor(0); }
                        _ => {}
                    }
                }
                enter_was_down = enter;

                let mut flags = 0i32;
                let mut mdx = 0.0f32;
                let mut mdy = 0.0f32;
                if game::mode() == 1 {
                    if (GetAsyncKeyState('W' as i32) as u16 & 0x8000) != 0 { flags |= 1; }
                    if (GetAsyncKeyState('S' as i32) as u16 & 0x8000) != 0 { flags |= 2; }
                    if (GetAsyncKeyState('A' as i32) as u16 & 0x8000) != 0 { flags |= 4; }
                    if (GetAsyncKeyState('D' as i32) as u16 & 0x8000) != 0 { flags |= 8; }
                    if (GetAsyncKeyState(0x10) as u16 & 0x8000) != 0 { flags |= 16; }
                    if (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 { flags |= 32; }
                    if (GetAsyncKeyState(0x20) as u16 & 0x8000) != 0 { flags |= 64; }
                    if (GetAsyncKeyState(0x1B) as u16 & 0x8000) != 0 {
                        game::set_mode(2);
                        ShowCursor(1);
                    } else {
                        let mut r: Rect = mem::zeroed();
                        GetClientRect(hwnd, &mut r);
                        let mut p: Point = mem::zeroed();
                        GetCursorPos(&mut p);
                        let cx = (r.left + r.right) / 2;
                        let cy = (r.top + r.bottom) / 2;
                        mdx = (p.x - cx) as f32;
                        mdy = (p.y - cy) as f32;
                        SetCursorPos(cx, cy);
                    }
                }
                game::step(dt, flags, mdx, mdy);

                let mut rect: Rect = mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                glViewport(0, 0, w, h);
                glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

                let p = &*(game::player_ptr() as *const game::Player);
                let aspect = w.max(1) as f32 / h.max(1) as f32;
                let near = 0.08f64;
                let top = near * (75.0f32.to_radians() / 2.0).tan() as f64;
                glMatrixMode(GL_PROJECTION);
                glLoadIdentity();
                glFrustum(-top * aspect as f64, top * aspect as f64, -top, top, near, 500.0);
                glMatrixMode(GL_MODELVIEW);
                glLoadIdentity();
                glRotatef(p.pitch * 57.2958, 1.0, 0.0, 0.0);
                glRotatef(p.yaw * 57.2958, 0.0, 1.0, 0.0);
                glTranslatef(-p.x, -p.y, -p.z);
                draw_world(p.x, p.z);

                if game::mode() == 1 {
                    draw_sword(p.attack);
                }
                draw_hud(w, h);
                SwapBuffers(dc);
            }
        }
    }
}

#[cfg(windows)]
fn main() {
    app::run();
}
