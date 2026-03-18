// struct Uniforms {
//     model: mat4x4f,
//     view: mat4x4f,
//     proj: mat4x4f,
//     normal: mat3x3f,
// }
// 
// @group(0) @binding(0) var<uniform> uniforms: Uniforms;
// 
// struct VertexInput {
//     @location(0) pos: vec3f,
//     @location(1) normal: vec3f,
// };
// 
// struct VertexOutput {
//     @builtin(position) pos: vec4f,
//     @location(1) normal: vec3f,
// }
// 
// @vertex
// fn vs_main(vertex: VertexInput) -> VertexOutput {
//     let pos = vec4f(vertex.pos, 1.0);
// 
//     var out: VertexOutput;
//     out.pos = uniforms.proj * uniforms.view * uniforms.model * pos;
//     out.normal = uniforms.normal * vertex.normal;
//     return out;
// }
// 
// const light_dir: vec3f = vec3f(1.0);
// const ambient_color: vec3f = vec3f(0.1);
// const diffuse_color: vec3f = vec3f(0.6);
// const specular_color: vec3f = vec3f(0.9);
// const frag_color: vec4f = vec4f(vec3f(0.75), 1.0);
// 
// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4f {
//     let diffuse = max(dot(normalize(in.normal), normalize(light_dir)), 0.0);
// 
//     let camera_dir = normalize(-in.pos.xyz);
//     let half_dir = normalize(camera_dir + normalize(light_dir));
//     let specular = pow(max(dot(half_dir, normalize(in.normal)), 0.0), 16.0);
// 
//     let lighting_color = ambient_color + diffuse * diffuse_color + specular * specular_color;
// 
//     return  frag_color * vec4f(lighting_color, 1.0);
// }

struct Uniforms {
    model: mat4x4f,
    view: mat4x4f,
    proj: mat4x4f,
    normal: mat3x3f,
    camera_pos: vec3f, // 1. Added the camera position here
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4f, // Renamed for clarity
    @location(0) world_pos: vec3f,      // 2. New field to pass world position
    @location(1) normal: vec3f,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    let local_pos = vec4f(vertex.pos, 1.0);
    var out: VertexOutput;
    
    // Calculate the position in world space
    let world_pos = uniforms.model * local_pos;
    
    out.world_pos = world_pos.xyz;
    out.clip_pos = uniforms.proj * uniforms.view * world_pos;
    
    // Note: Assuming uniforms.normal is the inverse-transpose of the model matrix
    out.normal = uniforms.normal * vertex.normal; 
    
    return out;
}

const light_dir: vec3f = vec3f(1.0);
const ambient_color: vec3f = vec3f(0.1);
const diffuse_color: vec3f = vec3f(0.6);
const specular_color: vec3f = vec3f(0.6);
const frag_color: vec4f = vec4f(vec3f(0.75), 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let n = normalize(in.normal);
    let l = normalize(light_dir);
    
    let diffuse = max(dot(n, l), 0.0);

    // 3. Calculate camera direction using world space coordinates
    let camera_dir = normalize(uniforms.camera_pos - in.world_pos);
    let half_dir = normalize(camera_dir + l);
    
    // Shininess factor of 16.0
    let specular = pow(max(dot(n, half_dir), 0.0), 8.0);

    let lighting_color = ambient_color + (diffuse * diffuse_color) + (specular * specular_color);

    return frag_color * vec4f(lighting_color, 1.0);
}
