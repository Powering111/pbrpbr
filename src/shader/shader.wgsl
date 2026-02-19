
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
    @location(1) world_pos: vec3f,
}

@group(0) @binding(0)
var<uniform> camera: mat4x4f;
@group(0) @binding(1)
var<uniform> camera_pos: vec3f;


const LIGHT_DIRECTIONAL:u32 = 1;
const LIGHT_POINT:u32 = 2;
const LIGHT_SPOT:u32 = 3;

struct Light {
    pos: vec3f,
    typ: u32,
    color: vec3f,
    intensity: f32,
}

@group(0) @binding(2)
var<uniform> lights: array<Light, 4>;


@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);
    let world_pos = model * vec4f(in.position, 1.0);
    out.position = camera * world_pos;
    out.world_pos = world_pos.xyz;

    let rot = mat3x3f(in.rot_0, in.rot_1, in.rot_2);
    out.normal = normalize(rot * in.normal);
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    var color = vec3f(0.0);
    for(var i = 0; i < 4; i++) {
        let light = lights[i];
        switch light.typ {
            case 1: {
                let light_in = in.world_pos - light.pos;
                
                let light_distance = length(light_in);
                dot(in.normal, normalize(light_in));
                let light_power =  0.0001 * light.intensity / (light_distance * light_distance);
                
                let light_out = camera_pos - in.world_pos;

                let halfway = (normalize(-light_in) + normalize(light_out));
                let specular = max(dot(halfway, in.normal), 0.0) * light_power;
                
                color += light.color * specular;
            }
            default: {

            }
        }
    }

    return vec4f(color, 1.0);
}