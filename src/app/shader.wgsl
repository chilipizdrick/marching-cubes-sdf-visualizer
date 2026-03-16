struct Uniforms {
    model: mat4x4f,
    view: mat4x4f,
    proj: mat4x4f,
    normal: mat3x3f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
};

struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) normal: vec3f,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    let pos = vec4f(vertex.pos, 1.0);

    var out: VertexOutput;
    out.pos = uniforms.proj * uniforms.view * uniforms.model * pos;
    out.normal = uniforms.normal * vertex.normal;
    return out;
}

const light_dir: vec3f = vec3f(1.0);
const ambient_color: vec3f = vec3f(0.1);
const diffuse_color: vec3f = vec3f(0.6);
const specular_color: vec3f = vec3f(0.9);
const frag_color: vec4f = vec4f(vec3f(0.75), 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let diffuse = max(dot(normalize(in.normal), normalize(light_dir)), 0.0);

    let camera_dir = normalize(-in.pos.xyz);
    let half_dir = normalize(camera_dir + normalize(light_dir));
    let specular = pow(max(dot(half_dir, normalize(in.normal)), 0.0), 16.0);

    let lighting_color = ambient_color + diffuse * diffuse_color + specular * specular_color;
    // let lighting_color = vec4f(ambient_color + diffuse * diffuse_color + specular * specular_color, 1.0);

    return  frag_color * vec4f(lighting_color, 1.0);
}

fn mat4x4_to_mat3x3(m: mat4x4f) -> mat3x3f {
    return mat3x3f(
        m[0].xyz,
        m[1].xyz,
        m[2].xyz,
    );
}
