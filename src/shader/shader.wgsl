
struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,

    @location(2) model_0: vec4f,
    @location(3) model_1: vec4f,
    @location(4) model_2: vec4f,
    @location(5) model_3: vec4f,
    @location(6) rot_0: vec3f,
    @location(7) rot_1: vec3f,
    @location(8) rot_2: vec3f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
}

@group(0) @binding(0)
var<uniform> camera: mat4x4f;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);
    var position = camera * model * vec4f(in.position, 1.0);
    out.position = position;

    let rot = mat3x3f(in.rot_0, in.rot_1, in.rot_2);
    out.normal = rot * in.normal;
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return vec4f(in.normal * 0.5 + vec3f(0.5), 1.0);
}