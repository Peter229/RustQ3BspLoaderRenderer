use std::io;
use std::io::prelude::*;
use std::fs::File;
use wgpu::util::DeviceExt;
use std::io::{stdin,stdout,Write};

use crate::texture;
use crate::bsp_look_up;

const PLANE_SIZE: u32 = 16;
const NODE_SIZE: u32 = 36;
const LEAF_SIZE: u32 = 48;
const LEAF_FACE_SIZE: u32 = 4;
const LEAF_BRUSH_SIZE: u32 = 4;
const BRUSH_SIZE: u32 = 12;
const BRUSH_SIDE_SIZE: u32 = 8;
const VERTEX_SIZE: u32 = 44;
const MESH_VERT_SIZE: u32 = 4;
const FACE_SIZE: u32 = 104;
const LIGHT_MAP_SIZE: u32 = 49152;
const LIGHT_VOL_SIZE: u32 = 8;
const TEXTURE_SIZE: u32 = 72;
const EFFECT_SIZE: u32 = 72;

const EPSILON: f32 = 0.03125;

const POLYGON: i32 = 1;
const PATCH: i32 = 2;
const MESH: i32 = 3;
const BILLBOARD: i32 = 4;
const BEZIER_LEVEL: i32 = 5;

const RAY: i32 = 0;
const SPHERE: i32 = 1;
const BOX: i32 = 2;

//http://www.mralligator.com/q3/#Nodes
//https://web.archive.org/web/20071010003301/http://www.devmaster.net/articles/quake3collision/

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VisData {
    num_vecs: i32,
    size_vecs: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightVol {
    ambient: [u8; 3],
    directional: [u8; 3],
    dir: [u8; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightMap {
    map: [[[u8; 3]; 128]; 128],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Face {
    texture: i32,
    effect: i32,
    type_draw: i32,
    vertex: i32,
    num_vertexes: i32,
    mesh_vert: i32,
    num_mesh_verts: i32,
    lightmap_index: i32,
    lightmap_start: [i32; 2],
    lightmap_size: [i32; 2],
    lightmap_origin: [f32; 3],
    lightmap_vecs: [[f32; 3]; 2],
    normal: [f32; 3],
    size: [i32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Effect {
    name: [u8; 64],
    brush: i32,
    unknown: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVert {
    offset: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    texcoord_s: [f32; 2],
    texcoord_l: [f32; 2],
    normal: [f32; 3],
    colour: [u8; 4],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uchar4Norm,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushSide {
    plane: i32,
    texture: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Brush {
    brush_side: i32,
    num_brush_sides: i32,
    texture: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Model {
    mins: [i32; 3],
    maxs: [i32; 3],
    face: i32,
    num_faces: i32,
    brush: i32,
    num_brushes: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LeafBrush {
    brush: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LeafFace {
    face: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Leaf {
    cluster: i32,
    area: i32,
    mins: [i32; 3],
    maxs: [i32; 3],
    leaf_face: i32,
    num_leaf_faces: i32,
    leaf_brush: i32,
    num_leaf_brushes: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Node {
    plane: i32,
    children: [i32; 2],
    mins: [i32; 3],
    maxs: [i32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Plane {
    normal: [f32; 3],
    distance: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Texture {
    name: [u8; 64],
    flags: i32,
    contents: i32,
}

pub struct Material {
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Trace {
    output_fraction: f32,
    output_end: cgmath::Vector3<f32>,
    output_starts_out: bool,
    output_all_solid: bool,
    start: cgmath::Vector3<f32>,
    end: cgmath::Vector3<f32>,
    radius: f32,
    mins: cgmath::Vector3<f32>,
    maxs: cgmath::Vector3<f32>,
    extents: cgmath::Vector3<f32>,
    t_type: i32,
}

impl Trace {

    pub fn new() -> Trace {
        Trace { output_fraction: 1.0, output_end: cgmath::Vector3::new(0.0, 0.0, 0.0), output_starts_out: true, output_all_solid: false, start: cgmath::Vector3::new(0.0, 0.0, 0.0), 
            end: cgmath::Vector3::new(0.0, 0.0, 0.0), radius: 1.0, mins: cgmath::Vector3::new(0.0, 0.0, 0.0), maxs: cgmath::Vector3::new(2.0, 2.0, 2.0), extents: cgmath::Vector3::new(1.0, 1.0, 1.0),
            t_type: RAY }
    }
}

pub struct Bsp {
    planes: Vec<Plane>,
    nodes: Vec<Node>,
    leafs: Vec<Leaf>,
    leaf_faces: Vec<LeafFace>,
    leaf_brushes: Vec<LeafBrush>,
    brushes: Vec<Brush>,
    brush_sides: Vec<BrushSide>,
    vertexes: Vec<Vertex>,
    mesh_verts: Vec<MeshVert>,
    faces: Vec<Face>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    light_maps: Vec<LightMap>,
    light_vols: Vec<LightVol>,
    t_trace: Trace,
    pub indices_per_texture: Vec<Vec<Vec<u32>>>,
    pub materials: Vec<Material>,
    textures: Vec<Texture>,
    pub materials_light: Vec<Material>,
}

impl Bsp {

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, light_layout: &wgpu::BindGroupLayout) -> Bsp {

        let mut planes: Vec<Plane> = Vec::new();
        let mut nodes: Vec<Node> = Vec::new();
        let mut leafs: Vec<Leaf> = Vec::new();
        let mut leaf_faces: Vec<LeafFace> = Vec::new();
        let mut leaf_brushes: Vec<LeafBrush> = Vec::new();
        let mut brushes: Vec<Brush> = Vec::new();
        let mut brush_sides: Vec<BrushSide> = Vec::new();
        let mut vertexes: Vec<Vertex> = Vec::new();
        let mut mesh_verts: Vec<MeshVert> = Vec::new();
        let mut faces: Vec<Face> = Vec::new();
        let mut light_maps: Vec<LightMap> = Vec::new();
        let mut light_vols: Vec<LightVol> = Vec::new();
        let mut textures: Vec<Texture> = Vec::new();
        let mut effects: Vec<Effect> = Vec::new();

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let mut s=String::new();
        print!("Please enter some text: ");
        let _=stdout().flush();
        stdin().read_line(&mut s).expect("Did not enter a correct string");
        if let Some('\n')=s.chars().next_back() {
            s.pop();
        }
        if let Some('\r')=s.chars().next_back() {
            s.pop();
        }

        let mut baseq3_pak0 = "baseq3/pak0.pk3".to_string();
        let mut f = std::fs::File::open(res_dir.join(baseq3_pak0)).unwrap();
        let mut reader = std::io::BufReader::new(f);
        let mut zip = zip::ZipArchive::new(reader).unwrap();

        let mut map = "maps/".to_string();
        map.push_str(&s);
        map.push_str(".bsp");
        let bytes = zip.by_name(&map).unwrap().bytes().map(|x| x.unwrap()).collect::<Vec<u8>>();

        //let bytes = std::fs::read(res_dir.join(s)).unwrap();

        //Check that it is a bsp file
        if bytes[0] == 'I' as u8 && bytes[1] == 'B' as u8 && bytes[2] == 'S' as u8 && bytes[3] == 'P' as u8 {
            
            //Get version of bsp
            let bsp_version = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[4], bytes[5], bytes[6], bytes[7]]) }.to_le();

            //Entities
            let entities_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[8], bytes[9], bytes[10], bytes[11]]) }.to_le();
            let entities_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[12], bytes[13], bytes[14], bytes[15]]) }.to_le();
            
            let mut entites: String = String::new();
            for i in entities_offset..(entities_length + entities_offset) {
                entites.push(bytes[i as usize] as char);
            }
            
            //Textures
            let textures_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[16], bytes[17], bytes[18], bytes[19]]) }.to_le();
            let textures_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[20], bytes[21], bytes[22], bytes[23]]) }.to_le();
            
            for i in 0..(textures_length / TEXTURE_SIZE) {
                let mut temp: [u8; TEXTURE_SIZE as usize] = [0; TEXTURE_SIZE as usize];
                for j in 0..TEXTURE_SIZE {
                    temp[j as usize] = bytes[(textures_offset + (i * TEXTURE_SIZE) + j) as usize];
                }
                let texture = bytemuck::from_bytes::<Texture>(&temp).clone();
                textures.push(texture);
            }

            //Planes
            let planes_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[24], bytes[25], bytes[26], bytes[27]]) }.to_le();
            let planes_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[28], bytes[29], bytes[30], bytes[31]]) }.to_le();

            for i in 0..(planes_length / PLANE_SIZE) {
                let mut temp: [u8; PLANE_SIZE as usize] = [0; PLANE_SIZE as usize];
                for j in 0..PLANE_SIZE {
                    temp[j as usize] = bytes[(planes_offset + (i * PLANE_SIZE) + j) as usize];
                }
                let plane = bytemuck::from_bytes::<Plane>(&temp).clone();
                planes.push(plane);
            }


            //Nodes
            let nodes_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[32], bytes[33], bytes[34], bytes[35]]) }.to_le();
            let nodes_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[36], bytes[37], bytes[38], bytes[39]]) }.to_le();
            
            for i in 0..(nodes_length / NODE_SIZE) {
                let mut temp: [u8; NODE_SIZE as usize] = [0; NODE_SIZE as usize];
                for j in 0..NODE_SIZE {
                    temp[j as usize] = bytes[(nodes_offset + (i * NODE_SIZE) + j) as usize];
                }
                let node = bytemuck::from_bytes::<Node>(&temp).clone();
                nodes.push(node);
            }

            //Leafs
            let leafs_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[40], bytes[41], bytes[42], bytes[43]]) }.to_le();
            let leafs_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[44], bytes[45], bytes[46], bytes[47]]) }.to_le();

            for i in 0..(leafs_length / LEAF_SIZE) {
                let mut temp: [u8; LEAF_SIZE as usize] = [0; LEAF_SIZE as usize];
                for j in 0..LEAF_SIZE {
                    temp[j as usize] = bytes[(leafs_offset + (i * LEAF_SIZE) + j) as usize];
                }
                let leaf = bytemuck::from_bytes::<Leaf>(&temp).clone();
                leafs.push(leaf);
            }

            //Leaf faces
            let leaf_faces_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[48], bytes[49], bytes[50], bytes[51]]) }.to_le();
            let leaf_faces_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[52], bytes[53], bytes[54], bytes[55]]) }.to_le();

            for i in 0..(leaf_faces_length / LEAF_FACE_SIZE) {
                let mut temp: [u8; LEAF_FACE_SIZE as usize] = [0; LEAF_FACE_SIZE as usize];
                for j in 0..LEAF_FACE_SIZE {
                    temp[j as usize] = bytes[(leaf_faces_offset + (i * LEAF_FACE_SIZE) + j) as usize];
                }
                let leaf_face = bytemuck::from_bytes::<LeafFace>(&temp).clone();
                leaf_faces.push(leaf_face);
            }

            //Leaf brushes
            let leaf_brushes_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[56], bytes[57], bytes[58], bytes[59]]) }.to_le();
            let leaf_brushes_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[60], bytes[61], bytes[62], bytes[63]]) }.to_le();

            for i in 0..(leaf_brushes_length / LEAF_BRUSH_SIZE) {
                let mut temp: [u8; LEAF_BRUSH_SIZE as usize] = [0; LEAF_BRUSH_SIZE as usize];
                for j in 0..LEAF_BRUSH_SIZE {
                    temp[j as usize] = bytes[(leaf_brushes_offset + (i * LEAF_BRUSH_SIZE) + j) as usize];
                }
                let leaf_brush = bytemuck::from_bytes::<LeafBrush>(&temp).clone();
                leaf_brushes.push(leaf_brush);
            }

            //Models 
            let models_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[64], bytes[65], bytes[66], bytes[67]]) }.to_le();
            let models_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[68], bytes[69], bytes[70], bytes[71]]) }.to_le();

            //Brushes
            let brushes_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[72], bytes[73], bytes[74], bytes[75]]) }.to_le();
            let brushes_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[76], bytes[77], bytes[78], bytes[79]]) }.to_le();

            for i in 0..(brushes_length / BRUSH_SIZE) {
                let mut temp: [u8; BRUSH_SIZE as usize] = [0; BRUSH_SIZE as usize];
                for j in 0..BRUSH_SIZE {
                    temp[j as usize] = bytes[(brushes_offset + (i * BRUSH_SIZE) + j) as usize];
                }
                let brush = bytemuck::from_bytes::<Brush>(&temp).clone();
                brushes.push(brush);
            }

            //Brush sides
            let brush_sides_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[80], bytes[81], bytes[82], bytes[83]]) }.to_le();
            let brush_sides_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[84], bytes[85], bytes[86], bytes[87]]) }.to_le();

            for i in 0..(brush_sides_length / BRUSH_SIDE_SIZE) {
                let mut temp: [u8; BRUSH_SIDE_SIZE as usize] = [0; BRUSH_SIDE_SIZE as usize];
                for j in 0..BRUSH_SIDE_SIZE {
                    temp[j as usize] = bytes[(brush_sides_offset + (i * BRUSH_SIDE_SIZE) + j) as usize];
                }
                let brush_side = bytemuck::from_bytes::<BrushSide>(&temp).clone();
                brush_sides.push(brush_side);
            }


            //Vertexes
            let vertexes_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[88], bytes[89], bytes[90], bytes[91]]) }.to_le();
            let vertexes_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[92], bytes[93], bytes[94], bytes[95]]) }.to_le();

            for i in 0..(vertexes_length / VERTEX_SIZE) {
                let mut temp: [u8; VERTEX_SIZE as usize] = [0; VERTEX_SIZE as usize];
                for j in 0..VERTEX_SIZE {
                    temp[j as usize] = bytes[(vertexes_offset + (i * VERTEX_SIZE) + j) as usize];
                }
                let vertex = bytemuck::from_bytes::<Vertex>(&temp).clone();
                vertexes.push(vertex);
            }

            //Mesh verts
            let mesh_verts_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[96], bytes[97], bytes[98], bytes[99]]) }.to_le();
            let mesh_verts_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[100], bytes[101], bytes[102], bytes[103]]) }.to_le();

            for i in 0..(mesh_verts_length / MESH_VERT_SIZE) {
                let mut temp: [u8; MESH_VERT_SIZE as usize] = [0; MESH_VERT_SIZE as usize];
                for j in 0..MESH_VERT_SIZE {
                    temp[j as usize] = bytes[(mesh_verts_offset + (i * MESH_VERT_SIZE) + j) as usize];
                }
                let mesh_vert = bytemuck::from_bytes::<MeshVert>(&temp).clone();
                mesh_verts.push(mesh_vert);
            }

            //Effects
            let effects_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[104], bytes[105], bytes[106], bytes[107]]) }.to_le();
            let effects_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[108], bytes[109], bytes[110], bytes[111]]) }.to_le();

            for i in 0..(effects_length / EFFECT_SIZE) {
                let mut temp: [u8; EFFECT_SIZE as usize] = [0; EFFECT_SIZE as usize];
                for j in 0..EFFECT_SIZE {
                    temp[j as usize] = bytes[(effects_offset + (i * EFFECT_SIZE) + j) as usize];
                }
                let effect = bytemuck::from_bytes::<Effect>(&temp).clone();
                effects.push(effect);
            }


            //Faces
            let faces_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[112], bytes[113], bytes[114], bytes[115]]) }.to_le();
            let faces_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[116], bytes[117], bytes[118], bytes[119]]) }.to_le();

            for i in 0..(faces_length / FACE_SIZE) {
                let mut temp: [u8; FACE_SIZE as usize] = [0; FACE_SIZE as usize];
                for j in 0..FACE_SIZE {
                    temp[j as usize] = bytes[(faces_offset + (i * FACE_SIZE) + j) as usize];
                }
                let face = bytemuck::from_bytes::<Face>(&temp).clone();
                faces.push(face);
            }

            //Lightmaps
            let lightmaps_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[120], bytes[121], bytes[122], bytes[123]]) }.to_le();
            let lightmaps_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[124], bytes[125], bytes[126], bytes[127]]) }.to_le();

            for i in 0..(lightmaps_length / LIGHT_MAP_SIZE) {
                let mut temp: [u8; LIGHT_MAP_SIZE as usize] = [0; LIGHT_MAP_SIZE as usize];
                for j in 0..LIGHT_MAP_SIZE {
                    temp[j as usize] = bytes[(lightmaps_offset + (i * LIGHT_MAP_SIZE) + j) as usize];
                }
                let light_map = bytemuck::from_bytes::<LightMap>(&temp).clone();
                light_maps.push(light_map);
            }

            //Lightvols
            let lightvols_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[128], bytes[129], bytes[130], bytes[131]]) }.to_le();
            let lightvols_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[132], bytes[133], bytes[134], bytes[135]]) }.to_le();

            for i in 0..(lightvols_length / LIGHT_VOL_SIZE) {
                let mut temp: [u8; LIGHT_VOL_SIZE as usize] = [0; LIGHT_VOL_SIZE as usize];
                for j in 0..LIGHT_VOL_SIZE {
                    temp[j as usize] = bytes[(lightvols_offset + (i * LIGHT_VOL_SIZE) + j) as usize];
                }
                let light_vol = bytemuck::from_bytes::<LightVol>(&temp).clone();
                light_vols.push(light_vol);
            }

            //Visdata
            let visdata_offset = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[136], bytes[137], bytes[138], bytes[139]]) }.to_le();
            let visdata_length = unsafe { std::mem::transmute::<[u8; 4], u32>([bytes[140], bytes[141], bytes[142], bytes[143]]) }.to_le();
        }
        //End of loading

        for i in 0..effects.len() {
            println!("{:?}", std::str::from_utf8(&effects[i].name).unwrap().chars().filter(|c| *c != 0 as char).collect::<String>());
        }

        //Start of mesh building
        let mut indices_per_texture: Vec<Vec<Vec<u32>>> = vec![vec![Vec::new(); textures.len()]; light_maps.len() + 1];
        for i in 0..(faces.len()) {
            
            let mut li = faces[i].lightmap_index as usize;
            if li >= light_maps.len() || li < 0 {
                li = light_maps.len();
                if faces[i].num_mesh_verts > 0 {
                    println!("Light map index {} Texture index {} Effect {}", faces[i].lightmap_index, faces[i].texture, faces[i].effect);
                    println!("{}", std::str::from_utf8(&textures[faces[i].texture as usize].name).unwrap().chars().filter(|c| *c != 0 as char).collect::<String>());
                }
            }

            if faces[i].type_draw == POLYGON {
                for j in 0..(faces[i].num_mesh_verts) {
                    indices_per_texture[li][faces[i].texture as usize].push((faces[i as usize].vertex + mesh_verts[(faces[i as usize].mesh_vert + j) as usize].offset) as u32);
                }
            }
            else if faces[i].type_draw == PATCH {
                
                //https://github.com/mikezila/uQuake3/blob/master/uQuake/Scripts/uQuake/GenerateMap.cs
                //https://github.com/mikezila/uQuake3/blob/master/uQuake/Scripts/uQuake/Types/BezierMesh.cs
                let num_patches = ((faces[i].size[0] - 1) / 2) * ((faces[i].size[1] - 1) / 2);
                for j in 0..num_patches {
                    let (i_vertexes, i_inds) = Bsp::gen_bez_mesh(&faces[i], j, &vertexes);

                    let offset = vertexes.len() as u32;
                    for l in 0..i_vertexes.len() {
                        vertexes.push(i_vertexes[l]);
                    }
                    for l in 0..i_inds.len() {
                        indices_per_texture[li][faces[i].texture as usize].push(offset + i_inds[l]);
                    }
                }
            }
            else if faces[i].type_draw == MESH {
                for j in 0..(faces[i].num_mesh_verts) {
                    indices_per_texture[li][faces[i].texture as usize].push((faces[i as usize].vertex + mesh_verts[(faces[i as usize].mesh_vert + j) as usize].offset) as u32);
                }
            }
            else if faces[i].type_draw == BILLBOARD {
                //Todo
            }
        }

        //Mesh building
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertexes),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );

        let mut indices_p_t: Vec<u32> = Vec::new();
        for j in 0..indices_per_texture.len() {
            for i in 0..indices_per_texture[j].len() {
                for l in 0..indices_per_texture[j][i].len() {
                    indices_p_t.push(indices_per_texture[j][i][l]);
                }
            }
        }

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices_p_t),
                usage: wgpu::BufferUsage::INDEX,
            }
        );

        //Lightmaps
        let mut all_light_maps: Vec<[[[u8; 4]; 128]; 128]> = Vec::new();
        let mut materials_light: Vec<Material> = Vec::new();
        for i in 0..light_maps.len() {
            let tex = texture::Texture::from_array(device, queue, bytemuck::bytes_of::<LightMap>(&light_maps[i]), 128, "lightmaps").unwrap();

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: light_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&tex.sampler),
                    },
                ],
                label: None,
            });

            materials_light.push(Material { diffuse_texture: tex, bind_group });
        }

        let mut materials: Vec<Material> = Vec::new();

        for i in 0..textures.len() {
            let tex_t = texture::Texture::load(device, queue, res_dir.join("debug.jpg")).unwrap();

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tex_t.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&tex_t.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material { diffuse_texture: tex_t, bind_group });
        }

        //Textures
        Bsp::load_from_pak("pak0.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak1.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak2.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak3.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak4.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak5.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak6.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak7.pk3", &textures, &mut materials, device, queue, layout);
        Bsp::load_from_pak("pak8.pk3", &textures, &mut materials, device, queue, layout);


        let t_trace = Trace::new();
        Bsp { planes, nodes, leafs, leaf_faces, leaf_brushes, brushes, brush_sides, vertexes, mesh_verts, faces, vertex_buffer, 
            index_buffer, light_maps, light_vols, t_trace, indices_per_texture, materials, textures, materials_light }
    }

    //Player Clipping
    pub fn trace_ray(&mut self, start: cgmath::Vector3<f32>, end: cgmath::Vector3<f32>) {

        self.t_trace = Trace { output_fraction: 1.0, output_end: cgmath::Vector3::new(0.0, 0.0, 0.0), output_starts_out: true, output_all_solid: false, start, 
            end, radius: 1.0, mins: cgmath::Vector3::new(0.0, 0.0, 0.0), maxs: cgmath::Vector3::new(2.0, 2.0, 2.0), extents: cgmath::Vector3::new(1.0, 1.0, 1.0),
            t_type: RAY };

        self.trace();
    }

    pub fn trace_sphere(&mut self, start: cgmath::Vector3<f32>, end: cgmath::Vector3<f32>, radius: f32) {

        self.t_trace = Trace { output_fraction: 1.0, output_end: cgmath::Vector3::new(0.0, 0.0, 0.0), output_starts_out: true, output_all_solid: false, start, 
            end, radius, mins: cgmath::Vector3::new(0.0, 0.0, 0.0), maxs: cgmath::Vector3::new(2.0, 2.0, 2.0), extents: cgmath::Vector3::new(1.0, 1.0, 1.0),
            t_type: SPHERE };

        self.trace();
    }

    pub fn trace_box(&mut self, start: cgmath::Vector3<f32>, end: cgmath::Vector3<f32>, mins: cgmath::Vector3<f32>, maxs: cgmath::Vector3<f32>) {

        let mut extents = cgmath::Vector3::new(0.0, 0.0, 0.0);

        if mins[0] == 0.0 && mins[1] == 0.0 && mins[2] == 0.0 &&
            maxs[0] == 0.0 && maxs[1] == 0.0 && maxs[2] == 0.0 {
                self.trace_ray(start, end);
        }
        else {
            if -mins[0] > maxs[0] {
                extents[0] = -mins[0];
            }
            else {
                extents[0] = -maxs[0];
            }
            
            if -mins[1] > maxs[1] {
                extents[1] = -mins[1];
            }
            else {
                extents[1] = -maxs[1];
            }

            if -mins[2] > maxs[2] {
                extents[2] = -mins[2];
            }
            else {
                extents[2] = -maxs[2];
            }
        }

        self.t_trace = Trace { output_fraction: 1.0, output_end: cgmath::Vector3::new(0.0, 0.0, 0.0), output_starts_out: true, output_all_solid: false, start, 
            end, radius: 1.0, mins, maxs, extents,
            t_type: BOX };

        self.trace();
    }


    fn trace(&mut self) {

        //self.trace = Trace { output_fraction: 1.0, output_end: cgmath::Vector3::new(0.0, 0.0, 0.0), output_starts_out: true, output_all_solid: false, start, end };
        let output_starts_out = true;
        let output_all_solid = false;
        let output_fraction = 1.0;

        self.check_node(0, 0.0, 1.0, self.t_trace.start, self.t_trace.end);

        if self.t_trace.output_fraction == 1.0 {
            self.t_trace.output_end = self.t_trace.end;
        }
        else {
            println!("COLLISION");
            for i in 0..3 {
                self.t_trace.output_end[i] = self.t_trace.start[i] + self.t_trace.output_fraction * (self.t_trace.end[i] - self.t_trace.start[i]);
            }
        }
    }

    fn check_node(&mut self, node_index: i32, start_fraction: f32, end_fraction: f32, start: cgmath::Vector3<f32>, end: cgmath::Vector3<f32>) {

        if node_index < 0 {
            let leaf = self.leafs[(-(node_index + 1)) as usize];
            for i in 0..leaf.num_leaf_brushes {
                let brush = self.brushes[self.leaf_brushes[(leaf.leaf_brush + i) as usize].brush as usize];
                //println!("{}", (self.textures[brush.texture as usize].flags) & 1);
                if brush.num_brush_sides > 0 && (self.textures[brush.texture as usize].contents & 1) == 1 {
                    self.check_brush(brush);
                }
            }

            return;
        }

        let node = self.nodes[node_index as usize];
        let plane = self.planes[node.plane as usize];

        let start_distance = cgmath::dot(start, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - plane.distance;
        let end_distance = cgmath::dot(end, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - plane.distance;
    
        let mut offset = 0.0;

        if self.t_trace.t_type == RAY {
            offset = 0.0;
        }
        else if self.t_trace.t_type == SPHERE {
            offset = self.t_trace.radius;
        }
        else if self.t_trace.t_type == BOX {
            offset = (self.t_trace.extents[0] * plane.normal[0]).abs() +
                    (self.t_trace.extents[1] * plane.normal[1]).abs() +
                    (self.t_trace.extents[2] * plane.normal[2]).abs();
        }

        if start_distance >= offset && end_distance >= offset {
            self.check_node(node.children[0], start_fraction, end_fraction, start, end);
        }
        else if start_distance < -offset && end_distance < -offset {
            self.check_node(node.children[1], start_fraction, end_fraction, start, end);
        }
        else {
            let mut side: i32 = 0;
            let mut fraction_1: f32 = 0.0;
            let mut fraction_2: f32 = 0.0;
            let mut middle_fraction: f32 = 0.0;
            let mut middle: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 0.0, 0.0);

            if start_distance < end_distance {
                side = 1;
                let inverse_distance = 1.0 / (start_distance - end_distance);
                fraction_1 = (start_distance - offset + EPSILON) * inverse_distance;
                fraction_2 = (start_distance + offset + EPSILON) * inverse_distance;
            }
            else if end_distance < start_distance {
                side = 0;
                let inverse_distance = 1.0 / (start_distance - end_distance);
                fraction_1 = (start_distance + offset + EPSILON) * inverse_distance;
                fraction_2 = (start_distance - offset - EPSILON) * inverse_distance;
            }
            else {
                side = 0;
                fraction_1 = 1.0;
                fraction_2 = 0.0;
            }

            if fraction_1 < 0.0 {
                fraction_1 = 0.0;
            }
            else if fraction_1 > 1.0 {
                fraction_1 = 1.0;
            }
            if fraction_2 < 0.0 {
                fraction_2 = 0.0;
            }
            else if fraction_2 > 1.0 {
                fraction_2 = 1.0;
            }

            middle_fraction = start_fraction + (end_fraction - start_fraction) * fraction_1;

            for i in 0..3 {
                middle[i] = start[i] + fraction_1 * (end[i] - start[i]);
            }

            self.check_node(node.children[side as usize].clone(), start_fraction, middle_fraction, start, middle);

            middle_fraction = start_fraction + (end_fraction - start_fraction) * fraction_2;

            for i in 0..3 {
                middle[i] = start[i] + fraction_2 * (end[i] - start[i]);
            }

            self.check_node(node.children[side as usize].clone(), middle_fraction, end_fraction, middle, end);
        }
    }

    fn check_brush(&mut self, brush: Brush) {

        let mut start_fraction = -1.0;
        let mut end_fraction = 1.0;
        let mut starts_out = false;
        let mut ends_out = false;

        for i in 0..brush.num_brush_sides {

            let brush_side = self.brush_sides[(brush.brush_side + i) as usize];
            let plane = self.planes[brush_side.plane as usize];

            let mut start_distance = 0.0;
            let mut end_distance = 0.0;

            if self.t_trace.t_type == RAY {
                start_distance = cgmath::dot(self.t_trace.start, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - plane.distance;
                end_distance = cgmath::dot(self.t_trace.end, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - plane.distance;
            }
            else if self.t_trace.t_type == SPHERE {
                start_distance = cgmath::dot(self.t_trace.start, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - (plane.distance + self.t_trace.radius);
                end_distance = cgmath::dot(self.t_trace.end, cgmath::Vector3::new(plane.normal[0], plane.normal[1], plane.normal[2])) - (plane.distance + self.t_trace.radius);
            }
            else if self.t_trace.t_type == BOX {

                let mut offset = cgmath::Vector3::new(0.0, 0.0, 0.0);
                for j in 0..3 {
                    if plane.normal[j] < 0.0 {
                        offset[j] = self.t_trace.maxs[j];
                    }
                    else {
                        offset[j] = self.t_trace.mins[j];
                    }

                    start_distance = (self.t_trace.start[0] + offset[0]) * plane.normal[0] +
                                    (self.t_trace.start[1] + offset[1]) * plane.normal[1] +
                                    (self.t_trace.start[2] + offset[2]) * plane.normal[2] -
                                    plane.distance;
                    
                    end_distance = (self.t_trace.end[0] + offset[0]) * plane.normal[0] +
                                    (self.t_trace.end[1] + offset[1]) * plane.normal[1] +
                                    (self.t_trace.end[2] + offset[2]) * plane.normal[2] -
                                    plane.distance;
                }
            }

            if start_distance > 0.0 {
                starts_out = true;
            }
            if end_distance > 0.0 {
                ends_out = true;
            }

            if start_distance > 0.0 && end_distance > 0.0 {
                return;
            }
            if start_distance <= 0.0 && end_distance <= 0.0 {
                continue;
            }

            if start_distance > end_distance {
                let fraction = (start_distance - EPSILON) / (start_distance - end_distance);
                if fraction > start_fraction {
                    start_fraction = fraction;
                }
            }
            else {
                let fraction = (start_distance + EPSILON) / (start_distance - end_distance);
                if fraction < end_fraction {
                    end_fraction = fraction;
                }
            }
        }

        if starts_out == false {
            self.t_trace.output_starts_out = false;
            if ends_out == false {
                self.t_trace.output_all_solid = true;
            }
            return;
        }

        if start_fraction < end_fraction {
            if start_fraction > -1.0 && start_fraction < self.t_trace.output_fraction {
                if start_fraction < 0.0 {
                    start_fraction = 0.0;
                }
                self.t_trace.output_fraction = start_fraction;
            }
        }
    }

    //Patch mesh builder
    fn bezier_curve(t: f32, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> [f32; 3] {

        let a = 1.0 - t;
        let tt = t * t;

        let mut t_points: [f32; 3] = [0.0; 3];
        for i in 0..3 {
            t_points[i] = ((a * a) * p0[i]) + (2.0 * a) * (t * p1[i]) + (tt * p2[i]);
        }

        t_points
    }

    fn bezier_curve_uv(t: f32, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> [f32; 2] {

        let a = 1.0 - t;
        let tt = t * t;

        let mut t_points: [f32; 2] = [0.0; 2];
        for i in 0..2 {
            t_points[i] = ((a * a) * p0[i]) + (2.0 * a) * (t * p1[i]) + (tt * p2[i]);
        }

        t_points
    }

    fn tessellate(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> Vec<[f32; 3]> {

        let mut vects: Vec<[f32; 3]> = Vec::new();
        
        let step_delta = 1.0 / (BEZIER_LEVEL as f32);
        let mut step = step_delta;

        vects.push(p0);
        for i in 0..(BEZIER_LEVEL - 1) {
            vects.push(Bsp::bezier_curve(step, p0, p1, p2));
            step += step_delta;
        }
        vects.push(p2);
        vects
    }

    fn tessellate_uv(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> Vec<[f32; 2]> {

        let mut uvs: Vec<[f32; 2]> = Vec::new();
        
        let step_delta = 1.0 / (BEZIER_LEVEL as f32);
        let mut step = step_delta;

        uvs.push(p0);
        for i in 0..(BEZIER_LEVEL - 1) {
            uvs.push(Bsp::bezier_curve_uv(step, p0, p1, p2));
            step += step_delta;
        }
        uvs.push(p2);
        uvs
    }

    fn gen_bezier_mesh(control_points: &Vec<Vertex>) -> (Vec<Vertex>, Vec<u32>) {

        let mut vertexes: Vec<Vertex> = Vec::new();
        let mut verts: Vec<Vertex> = Vec::new();
        let mut inds: Vec<u32> = Vec::new();

        let p0s = Bsp::tessellate(control_points[0].position, control_points[3].position, control_points[6].position);
        let p0s_uvs = Bsp::tessellate_uv(control_points[0].texcoord_s, control_points[3].texcoord_s, control_points[6].texcoord_s);
        let p0s_uvl = Bsp::tessellate_uv(control_points[0].texcoord_l, control_points[3].texcoord_l, control_points[6].texcoord_l);
        let p0s_col = Bsp::tessellate([control_points[0].colour[0] as f32, control_points[0].colour[1] as f32, control_points[0].colour[2] as f32], 
                                    [control_points[3].colour[0] as f32, control_points[3].colour[1] as f32, control_points[3].colour[2] as f32], 
                                    [control_points[6].colour[0] as f32, control_points[6].colour[1] as f32, control_points[6].colour[2] as f32]);

        let p1s = Bsp::tessellate(control_points[1].position, control_points[4].position, control_points[7].position);
        let p1s_uvs = Bsp::tessellate_uv(control_points[1].texcoord_s, control_points[4].texcoord_s, control_points[7].texcoord_s);
        let p1s_uvl = Bsp::tessellate_uv(control_points[1].texcoord_l, control_points[4].texcoord_l, control_points[7].texcoord_l);
        let p1s_col = Bsp::tessellate([control_points[1].colour[0] as f32, control_points[1].colour[1] as f32, control_points[1].colour[2] as f32], 
            [control_points[4].colour[0] as f32, control_points[4].colour[1] as f32, control_points[4].colour[2] as f32], 
            [control_points[7].colour[0] as f32, control_points[7].colour[1] as f32, control_points[7].colour[2] as f32]);

        let p2s = Bsp::tessellate(control_points[2].position, control_points[5].position, control_points[8].position);
        let p2s_uvs = Bsp::tessellate_uv(control_points[2].texcoord_s, control_points[5].texcoord_s, control_points[8].texcoord_s);
        let p2s_uvl = Bsp::tessellate_uv(control_points[2].texcoord_l, control_points[5].texcoord_l, control_points[8].texcoord_l);
        let p2s_col = Bsp::tessellate([control_points[4].colour[0] as f32, control_points[4].colour[1] as f32, control_points[4].colour[2] as f32], 
            [control_points[4].colour[0] as f32, control_points[4].colour[1] as f32, control_points[4].colour[2] as f32], 
            [control_points[8].colour[0] as f32, control_points[8].colour[1] as f32, control_points[8].colour[2] as f32]);
        
        for i in 0..(BEZIER_LEVEL+1) {
            let pfs = Bsp::tessellate(p0s[i as usize], p1s[i as usize], p2s[i as usize]);
            let pfs_uvs = Bsp::tessellate_uv(p0s_uvs[i as usize], p1s_uvs[i as usize], p2s_uvs[i as usize]);
            let pfs_uvl = Bsp::tessellate_uv(p0s_uvl[i as usize], p1s_uvl[i as usize], p2s_uvl[i as usize]);
            let pfs_col = Bsp::tessellate(p0s_col[i as usize], p1s_col[i as usize], p2s_col[i as usize]);
            for j in 0..pfs.len() {
                let pfs_col_u: [u8; 4] = [pfs_col[j][0].max(0.0).min(255.0) as u8, pfs_col[j][1].max(0.0).min(255.0) as u8, pfs_col[j][2].max(0.0).min(255.0) as u8, control_points[0].colour[3]];
                vertexes.push(Vertex { position: pfs[j as usize], texcoord_s: pfs_uvs[j as usize], texcoord_l: pfs_uvl[j as usize],
                    normal: control_points[0].normal, colour: pfs_col_u });
            }
        }

        let num_verts = ((BEZIER_LEVEL + 1) * (BEZIER_LEVEL + 1)) as usize;
        let mut x_step = 1;
        let width = (BEZIER_LEVEL + 1) as usize;
        for i in 0..(num_verts - width) {

            if x_step == 1 {
                inds.push(i as u32);
                inds.push((i + width) as u32);
                inds.push((i + 1) as u32);
                x_step += 1;
                continue;
            }
            else if (x_step == width) {
                inds.push(i as u32);
                inds.push((i + (width - 1)) as u32);
                inds.push((i + width) as u32);
                x_step = 1;
                continue;
            }
            else {
                inds.push(i as u32);
                inds.push((i + (width - 1)) as u32);
                inds.push((i + width) as u32);

                inds.push(i as u32);
                inds.push((i + width) as u32);
                inds.push((i + 1) as u32);
                x_step += 1;
                continue;
            }
        }

        (vertexes, inds)
    }

    fn gen_bez_mesh(face: &Face, patch_num: i32, vertexes: &Vec<Vertex>) -> (Vec<Vertex>, Vec<u32>) {

        let num_patches_x = ((face.size[0]) - 1) / 2;
        let num_patches_y = ((face.size[1]) - 1) / 2;
        let mut step_x = 0;
        let mut step_y = 0;
        for i in 0..patch_num {

            step_x += 1;
            if step_x == num_patches_x {

                step_x = 0;
                step_y += 1;
            }
        }

        let mut vert_grid: Vec<Vec<Vertex>> = vec![vec![Vertex { position: [0.0, 0.0, 0.0],
            texcoord_s: [0.0, 0.0],
            texcoord_l: [0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
            colour: [0u8, 0u8, 0u8, 255u8] }; face.size[1] as usize]; face.size[0] as usize];

        let mut grid_x_step = 0;
        let mut grid_y_step = 0;
        let mut vert_step = face.vertex;
        for i in 0..face.num_vertexes {
            vert_grid[grid_x_step][grid_y_step] = vertexes[vert_step as usize];
            vert_step += 1;
            grid_x_step += 1;
            if grid_x_step as i32 == face.size[0] {
                grid_x_step = 0;
                grid_y_step += 1;
            }
        }
        let vi = (2 * step_x) as usize;
        let vj = (2 * step_y) as usize;

        let mut b_verts: Vec<Vertex> = Vec::new();
        b_verts.push(vert_grid[vi][vj]);
        b_verts.push(vert_grid[vi + 1][vj]);
        b_verts.push(vert_grid[vi + 2][vj]);
        b_verts.push(vert_grid[vi][vj + 1]);
        b_verts.push(vert_grid[vi + 1][vj + 1]);
        b_verts.push(vert_grid[vi + 2][vj + 1]);
        b_verts.push(vert_grid[vi][vj + 2]);
        b_verts.push(vert_grid[vi + 1][vj + 2]);
        b_verts.push(vert_grid[vi + 2][vj + 2]);

        Bsp::gen_bezier_mesh(&b_verts)
    }

    //Loading from pak
    fn load_from_pak(pak: &str, textures: &Vec<Texture>, materials: &mut Vec<Material>, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) {

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");

        let mut baseq3_pak = "baseq3/".to_string();
        baseq3_pak.push_str(pak);

        let mut f = std::fs::File::open(res_dir.join(baseq3_pak)).unwrap();
        let mut reader = std::io::BufReader::new(f);
        
        let mut zip = zip::ZipArchive::new(reader).unwrap();

        for i in 0..textures.len() {

            let mut tex: String = std::str::from_utf8(&textures[i].name).unwrap().chars().filter(|c| *c != 0 as char).collect();
            let mut tex_j = tex.clone();
            tex_j.push_str(".jpg");

            let mut check_tga = false;
            let mut use_look_up = false;

            match zip.by_name(&tex_j) {
                Ok(file) => {
                    let tex = texture::Texture::from_bytes_format(device, queue, bytemuck::cast_slice(&(file.bytes().map(|x| x.unwrap()).collect::<Vec<u8>>())), image::ImageFormat::Jpeg, "Tex").unwrap();
                
                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&tex.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&tex.sampler),
                            },
                        ],
                        label: None,
                    });

                    materials[i] = Material { diffuse_texture: tex, bind_group };
                },
                Err(e) => {
                    check_tga = true;
                },
            };

            if check_tga {
                let mut tex_t = tex.clone();
                tex_t.push_str(".tga");

                match zip.by_name(&tex_t) {
                    Ok(file) => {
                        let tex = texture::Texture::from_bytes_format(device, queue, bytemuck::cast_slice(&(file.bytes().map(|x| x.unwrap()).collect::<Vec<u8>>())), image::ImageFormat::Tga, "Tex").unwrap();
                    
                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                },
                            ],
                            label: None,
                        });
    
                        materials[i] = Material { diffuse_texture: tex, bind_group };
                    },
                    Err(e) => {
                        //println!("Error cant find {}", tex);
                        use_look_up = true;
                    },
                };
            }

            //Look up request
            if use_look_up {
                tex = bsp_look_up::look_up_table(&tex);
                tex_j = tex.clone();
                tex_j.push_str(".jpg");

                check_tga = false;

                match zip.by_name(&tex_j) {
                    Ok(file) => {
                        let tex = texture::Texture::from_bytes_format(device, queue, bytemuck::cast_slice(&(file.bytes().map(|x| x.unwrap()).collect::<Vec<u8>>())), image::ImageFormat::Jpeg, "Tex").unwrap();
                    
                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                },
                            ],
                            label: None,
                        });

                        materials[i] = Material { diffuse_texture: tex, bind_group };
                    },
                    Err(e) => {
                        check_tga = true;
                    },
                };

                if check_tga {
                    let mut tex_t = tex.clone();
                    tex_t.push_str(".tga");

                    match zip.by_name(&tex_t) {
                        Ok(file) => {
                            let tex = texture::Texture::from_bytes_format(device, queue, bytemuck::cast_slice(&(file.bytes().map(|x| x.unwrap()).collect::<Vec<u8>>())), image::ImageFormat::Tga, "Tex").unwrap();
                        
                            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(&tex.view),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                    },
                                ],
                                label: None,
                            });
        
                            materials[i] = Material { diffuse_texture: tex, bind_group };
                        },
                        Err(e) => {
                            //println!("Error cant find {}", tex);
                        },
                    };
                }
            }
        }
    }
}