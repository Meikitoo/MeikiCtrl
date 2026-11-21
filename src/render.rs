// Rendu Direct3D 11 dans une swapchain DirectComposition (transparence réelle,
// alpha prémultiplié). Anticrénelage 4x MSAA. Aucune WebView, aucun moteur de jeu.

use std::mem::size_of;

use glam::{Mat4, Vec3};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use crate::model::Scene;

const MSAA: u32 = 4;
const SHADER_SRC: &str = include_str!("shader.hlsl");

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 3],
    nrm: [f32; 3],
}

#[repr(C)]
struct Constants {
    world: [f32; 16],
    view_proj: [f32; 16],
    normal_mat: [f32; 16],
    root_inv: [f32; 16],
    base_color: [f32; 4],
    emissive: [f32; 4],
    /// x = éclairs, y = temps, z = lueur, w = énergie
    params: [f32; 4],
    cam_pos: [f32; 4],
    /// rgb = couleur d'accent, w = opacité
    accent: [f32; 4],
}

#[derive(Clone, Copy, Default)]
struct MeshRange {
    start: u32,
    count: u32,
}

pub struct Renderer {
    ctx: ID3D11DeviceContext,
    swap: IDXGISwapChain1,
    _dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    backbuffer: ID3D11Texture2D,
    msaa_tex: ID3D11Texture2D,
    msaa_rtv: ID3D11RenderTargetView,
    dsv: ID3D11DepthStencilView,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    layout: ID3D11InputLayout,
    vb: ID3D11Buffer,
    ib: ID3D11Buffer,
    cb: ID3D11Buffer,
    raster: ID3D11RasterizerState,
    meshes: Vec<MeshRange>,
    width: u32,
    height: u32,
    /// Images sautées parce que Windows n'avait pas encore affiché la précédente.
    pub skipped: u32,
    /// Priorité GPU obtenue : « temps réel », « haute » ou « normale ».
    pub gpu_priority: &'static str,
}

impl Renderer {
    /// `extra` : pièces du modèle 3D (sommets position+normale, indices), ajoutées après les 3 formes simples.
    pub fn new(hwnd: HWND, width: u32, height: u32, extra: &[(&[[f32; 6]], &[u32])]) -> Result<Self> {
        unsafe {
            // --- Périphérique D3D11 ---
            let mut device = None;
            let mut ctx = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )?;
            let device: ID3D11Device = device.unwrap();
            let ctx: ID3D11DeviceContext = ctx.unwrap();

            // --- Swapchain de composition (transparente) ---
            let dxgi_device: IDXGIDevice = device.cast()?;
            // Une seule image en attente (au lieu de 3) : l'overlay suit la manette sans retard.
            if let Ok(d1) = dxgi_device.cast::<IDXGIDevice1>() {
                let _ = d1.SetMaximumFrameLatency(1);
            }
            // Priorité GPU maximale pour l'overlay (Windows peut refuser sans droits admin).
            let _ = dxgi_device.SetGPUThreadPriority(7);
            let gpu_priority = raise_gpu_priority();
            let adapter = dxgi_device.GetAdapter()?;
            let factory: IDXGIFactory2 = adapter.GetParent()?;
            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                ..Default::default()
            };
            let swap = factory.CreateSwapChainForComposition(&device, &desc, None)?;

            // --- DirectComposition : pose la swapchain sur la fenêtre ---
            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
            let target = dcomp.CreateTargetForHwnd(hwnd, TRUE)?;
            let visual = dcomp.CreateVisual()?;
            visual.SetContent(&swap)?;
            target.SetRoot(&visual)?;
            dcomp.Commit()?;

            let backbuffer: ID3D11Texture2D = swap.GetBuffer(0)?;

            // --- Cibles MSAA (couleur + profondeur) ---
            let mut tex_desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: MSAA, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut msaa_tex = None;
            device.CreateTexture2D(&tex_desc, None, Some(&mut msaa_tex))?;
            let msaa_tex = msaa_tex.unwrap();
            let mut msaa_rtv = None;
            device.CreateRenderTargetView(&msaa_tex, None, Some(&mut msaa_rtv))?;

            tex_desc.Format = DXGI_FORMAT_D24_UNORM_S8_UINT;
            tex_desc.BindFlags = D3D11_BIND_DEPTH_STENCIL.0 as u32;
            let mut depth_tex = None;
            device.CreateTexture2D(&tex_desc, None, Some(&mut depth_tex))?;
            let depth_tex = depth_tex.unwrap();
            let mut dsv = None;
            device.CreateDepthStencilView(&depth_tex, None, Some(&mut dsv))?;

            // --- Shaders ---
            let vsb = compile(s!("VSMain"), s!("vs_5_0"))?;
            let psb = compile(s!("PSMain"), s!("ps_5_0"))?;
            let mut vs = None;
            device.CreateVertexShader(blob_bytes(&vsb), None, Some(&mut vs))?;
            let mut ps = None;
            device.CreatePixelShader(blob_bytes(&psb), None, Some(&mut ps))?;

            let elems = [
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("NORMAL"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 12,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ];
            let mut layout = None;
            device.CreateInputLayout(&elems, blob_bytes(&vsb), Some(&mut layout))?;

            // --- Géométrie ---
            let (verts, idx, meshes) = build_meshes(extra);
            let vb = immutable_buffer(&device, &verts, D3D11_BIND_VERTEX_BUFFER)?;
            let ib = immutable_buffer(&device, &idx, D3D11_BIND_INDEX_BUFFER)?;
            let cb_desc = D3D11_BUFFER_DESC {
                ByteWidth: size_of::<Constants>() as u32,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let mut cb = None;
            device.CreateBuffer(&cb_desc, None, Some(&mut cb))?;

            let rd = D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_NONE,
                DepthClipEnable: TRUE,
                MultisampleEnable: TRUE,
                ..Default::default()
            };
            let mut raster = None;
            device.CreateRasterizerState(&rd, Some(&mut raster))?;

            Ok(Self {
                ctx,
                swap,
                _dcomp: dcomp,
                _target: target,
                _visual: visual,
                backbuffer,
                msaa_tex,
                msaa_rtv: msaa_rtv.unwrap(),
                dsv: dsv.unwrap(),
                vs: vs.unwrap(),
                ps: ps.unwrap(),
                layout: layout.unwrap(),
                vb,
                ib,
                cb: cb.unwrap(),
                raster: raster.unwrap(),
                meshes,
                width,
                height,
                skipped: 0,
                gpu_priority,
            })
        }
    }

    pub fn render(&mut self, scene: &Scene, time: f32) -> Result<()> {
        unsafe {
            let ctx = &self.ctx;
            ctx.OMSetRenderTargets(Some(&[Some(self.msaa_rtv.clone())]), &self.dsv);
            ctx.ClearRenderTargetView(&self.msaa_rtv, &[0.0f32; 4]);
            ctx.ClearDepthStencilView(&self.dsv, D3D11_CLEAR_DEPTH.0 as u32, 1.0, 0);
            ctx.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.width as f32,
                Height: self.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            ctx.RSSetState(&self.raster);
            ctx.IASetInputLayout(&self.layout);
            ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let stride = size_of::<Vertex>() as u32;
            let offset = 0u32;
            ctx.IASetVertexBuffers(0, 1, Some(&Some(self.vb.clone())), Some(&stride), Some(&offset));
            ctx.IASetIndexBuffer(&self.ib, DXGI_FORMAT_R32_UINT, 0);
            ctx.VSSetShader(&self.vs, None);
            ctx.PSSetShader(&self.ps, None);
            ctx.VSSetConstantBuffers(0, Some(&[Some(self.cb.clone())]));
            ctx.PSSetConstantBuffers(0, Some(&[Some(self.cb.clone())]));

            let aspect = self.width as f32 / self.height as f32;
            let view = Mat4::look_at_lh(scene.cam_pos, scene.cam_target, Vec3::Z);
            let proj = Mat4::perspective_lh(scene.fov_y, aspect, 0.1, 50.0);
            let view_proj = (proj * view).to_cols_array();
            let root_inv = scene.root.inverse().to_cols_array();
            let cam = scene.cam_pos;

            for d in &scene.draws {
                let c = Constants {
                    world: d.world.to_cols_array(),
                    view_proj,
                    normal_mat: d.world.inverse().transpose().to_cols_array(),
                    root_inv,
                    base_color: [d.color.x, d.color.y, d.color.z, 1.0],
                    emissive: [d.emissive.x, d.emissive.y, d.emissive.z, 0.0],
                    params: [if d.lightning { 1.0 } else { 0.0 }, time, d.glow, scene.energy],
                    cam_pos: [cam.x, cam.y, cam.z, 1.0],
                    accent: [scene.accent.x, scene.accent.y, scene.accent.z, scene.opacity],
                };
                ctx.UpdateSubresource(&self.cb, 0, None, &c as *const Constants as *const _, 0, 0);
                let m = self.meshes[d.mesh];
                ctx.DrawIndexed(m.count, m.start, 0);
            }

            ctx.ResolveSubresource(&self.backbuffer, 0, &self.msaa_tex, 0, DXGI_FORMAT_B8G8R8A8_UNORM);
            // Sans attendre l'écran : si Windows n'a pas encore affiché l'image précédente,
            // on saute celle-ci plutôt que de bloquer l'overlay.
            let hr = self.swap.Present(0, DXGI_PRESENT_DO_NOT_WAIT);
            if hr == DXGI_ERROR_WAS_STILL_DRAWING {
                self.skipped += 1;
                return Ok(());
            }
            hr.ok()
        }
    }
}

/// Même technique qu'OBS : demander à Windows de servir l'overlay AVANT le jeu sur le GPU.
/// Sans ça, un jeu à 100 % de GPU (ex. 540 FPS non limités) fait attendre l'overlay
/// des centaines de millisecondes. Le travail de l'overlay est minuscule : le jeu n'y perd presque rien.
fn raise_gpu_priority() -> &'static str {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let Ok(gdi) = LoadLibraryW(w!("gdi32.dll")) else {
            return "normale";
        };
        let Some(func) = GetProcAddress(gdi, s!("D3DKMTSetProcessSchedulingPriorityClass")) else {
            eprintln!("Priorité GPU : fonction Windows introuvable.");
            return "normale";
        };
        // NTSTATUS D3DKMTSetProcessSchedulingPriorityClass(HANDLE, D3DKMT_SCHEDULINGPRIORITYCLASS)
        let set: unsafe extern "system" fn(HANDLE, i32) -> i32 = std::mem::transmute(func);
        let me = GetCurrentProcess();
        // 5 = temps réel (droits admin requis), 4 = haute
        for (class, name) in [(5, "temps réel"), (4, "haute")] {
            if set(me, class) == 0 {
                println!("Priorité GPU de MeikiCtrl : {name}");
                return name;
            }
        }
        println!("Priorité GPU : refusée par Windows. Active le mode administrateur dans Système.");
        "normale"
    }
}

fn compile(entry: PCSTR, target: PCSTR) -> Result<ID3DBlob> {
    unsafe {
        let mut blob = None;
        let mut err = None;
        let r = D3DCompile(
            SHADER_SRC.as_ptr() as *const _,
            SHADER_SRC.len(),
            s!("shader.hlsl"),
            None,
            None,
            entry,
            target,
            1 << 15, // D3DCOMPILE_OPTIMIZATION_LEVEL3
            0,
            &mut blob,
            Some(&mut err),
        );
        if let Err(e) = r {
            if let Some(err) = err {
                eprintln!("Erreur de shader :\n{}", String::from_utf8_lossy(blob_bytes(&err)));
            }
            return Err(e);
        }
        Ok(blob.unwrap())
    }
}

fn blob_bytes(b: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()) }
}

fn immutable_buffer<T>(device: &ID3D11Device, data: &[T], bind: D3D11_BIND_FLAG) -> Result<ID3D11Buffer> {
    unsafe {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: std::mem::size_of_val(data) as u32,
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: bind.0 as u32,
            ..Default::default()
        };
        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: data.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut buf = None;
        device.CreateBuffer(&desc, Some(&init), Some(&mut buf))?;
        Ok(buf.unwrap())
    }
}

/// Sphère (rayon 1), cylindre (rayon 1, de y=0 à y=1), cube (côté 1, centré).
fn build_meshes(extra: &[(&[[f32; 6]], &[u32])]) -> (Vec<Vertex>, Vec<u32>, Vec<MeshRange>) {
    use std::f32::consts::PI;
    let mut v: Vec<Vertex> = Vec::new();
    let mut i: Vec<u32> = Vec::new();
    let mut ranges = vec![MeshRange::default(); 3];

    // Sphère
    let start = i.len() as u32;
    let base = v.len() as u32;
    let (lat, lon) = (20u32, 36u32);
    for a in 0..=lat {
        let th = PI * a as f32 / lat as f32;
        for b in 0..=lon {
            let ph = 2.0 * PI * b as f32 / lon as f32;
            let p = [th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()];
            v.push(Vertex { pos: p, nrm: p });
        }
    }
    for a in 0..lat {
        for b in 0..lon {
            let p0 = base + a * (lon + 1) + b;
            let p1 = p0 + lon + 1;
            i.extend_from_slice(&[p0, p1, p0 + 1, p0 + 1, p1, p1 + 1]);
        }
    }
    ranges[0] = MeshRange { start, count: i.len() as u32 - start };

    // Cylindre
    let start = i.len() as u32;
    let seg = 32u32;
    let base = v.len() as u32;
    for s in 0..=seg {
        let ph = 2.0 * PI * s as f32 / seg as f32;
        let (c, sn) = (ph.cos(), ph.sin());
        v.push(Vertex { pos: [c, 0.0, sn], nrm: [c, 0.0, sn] });
        v.push(Vertex { pos: [c, 1.0, sn], nrm: [c, 0.0, sn] });
    }
    for s in 0..seg {
        let a = base + s * 2;
        i.extend_from_slice(&[a, a + 1, a + 2, a + 2, a + 1, a + 3]);
    }
    for (y, ny) in [(0.0f32, -1.0f32), (1.0, 1.0)] {
        let center = v.len() as u32;
        v.push(Vertex { pos: [0.0, y, 0.0], nrm: [0.0, ny, 0.0] });
        for s in 0..=seg {
            let ph = 2.0 * PI * s as f32 / seg as f32;
            v.push(Vertex { pos: [ph.cos(), y, ph.sin()], nrm: [0.0, ny, 0.0] });
        }
        for s in 0..seg {
            i.extend_from_slice(&[center, center + 1 + s, center + 2 + s]);
        }
    }
    ranges[1] = MeshRange { start, count: i.len() as u32 - start };

    // Cube
    let start = i.len() as u32;
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (n, u, w) in faces {
        let b0 = v.len() as u32;
        for (su, sw) in [(-0.5f32, -0.5f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            let p = [
                n[0] * 0.5 + u[0] * su + w[0] * sw,
                n[1] * 0.5 + u[1] * su + w[1] * sw,
                n[2] * 0.5 + u[2] * su + w[2] * sw,
            ];
            v.push(Vertex { pos: p, nrm: n });
        }
        i.extend_from_slice(&[b0, b0 + 1, b0 + 2, b0, b0 + 2, b0 + 3]);
    }
    ranges[2] = MeshRange { start, count: i.len() as u32 - start };

    // Pièces du modèle 3D
    for (verts, inds) in extra {
        let start = i.len() as u32;
        let base = v.len() as u32;
        v.extend(verts.iter().map(|a| Vertex { pos: [a[0], a[1], a[2]], nrm: [a[3], a[4], a[5]] }));
        i.extend(inds.iter().map(|x| x + base));
        ranges.push(MeshRange { start, count: inds.len() as u32 });
    }

    (v, i, ranges)
}
