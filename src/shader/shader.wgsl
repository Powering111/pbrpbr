
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

const PI:f32 = 3.14159265;


struct Light {
    matrix: mat4x4f,
    pos: vec3f,
    typ: u32,
    color: vec3f,
    intensity: f32,
    direction: vec3f,
    extra1: f32,
    extra2: f32,
}

struct Material {
    base_color: vec4f,
    metallic: f32,
    roughness: f32,
}


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

@group(0) @binding(2)
var<uniform> lights: array<Light, 4>;

@group(1) @binding(0)
var fsampler: sampler;

@group(1) @binding(1)
var shadow_maps: texture_depth_2d_array;

@group(2) @binding(0)
var<uniform> material: Material;

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    var color = vec3f(0.0);

    let normal = in.normal;
    let light_out = camera_pos - in.world_pos;
    let view_dir = normalize(light_out);
    
    for(var i = 0; i < 4; i++) {
        let light = lights[i];
        let light_space_pos = into_vec3_pos(light.matrix * vec4f(in.world_pos, 1.0));
        let shadow_depth = textureSample(shadow_maps, fsampler, ndc_to_uv(light_space_pos.xy), i);
        if light_space_pos.z - 0.000001 > shadow_depth {
            continue;
        }
        
        switch light.typ {
            case 1: {
                // Point light
                let light_in = in.world_pos - light.pos;
                let light_dir = normalize(-light_in);

                let light_distance = length(light_in);
                let light_power = light.intensity / (light_distance * light_distance);
                color += brdf(light_dir, view_dir, normal) * light_power * max(dot(normal, light_dir), 0.0);
            }
            case 2: {
                // Directional light
                let light_in = light.direction;
                let light_dir = normalize(-light_in);

                let light_power = 0.2 * light.intensity;
                color += brdf(light_dir, view_dir, normal) * light_power * max(dot(normal, light_dir), 0.0);
            }
            case 3: {
                // Spot light
                let light_in = in.world_pos - light.pos;
                let light_dir = normalize(-light_in);

                let angle = acos(dot(light.direction,-light_dir));
                var falloff = 0.0;
                if angle < light.extra1 {
                    falloff = 1.0;
                }
                else if angle < light.extra2 {
                    falloff = (light.extra2 - angle) / (light.extra2 - light.extra1);
                }
                else {
                    break;
                }

                let light_distance = length(light_in);
                let light_power = 0.2 * light.intensity * falloff / (light_distance * light_distance);
                color += brdf(light_dir, view_dir, normal) * light_power * max(dot(normal, light_dir), 0.0);
            }
            default: {
                
            }
        }
    }

    // ambient
    color += 0.1 * material.base_color.xyz;

    color = tone_map(color);
    return vec4f(color, 1.0);
}

@vertex
fn vs_light(
    in: VertexInput,
) -> @builtin(position) vec4f {
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);
    let world_pos = model * vec4f(in.position, 1.0);
    return camera * world_pos;
}

@fragment
fn fs_light() -> @location(0) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

fn brdf(light_dir: vec3f, view_dir: vec3f, normal: vec3f) -> vec3f {
    let halfway = normalize(light_dir + view_dir);

    let roughness = material.roughness;
    let metallic = material.metallic;
    var albedo = material.base_color.xyz;
    
    let f_0 = mix(vec3f(0.4), albedo, metallic);
    let fresnel = f_0 + (1 - f_0) * pow((1 - dot(halfway, view_dir)), 5.0);

    let k_s = fresnel;
    let k_d = (vec3f(1.0) - k_s) * (1.0 - metallic);
    let diffuse = albedo / PI;

    let roughness2 = roughness * roughness;
    let distribution = roughness2 / (PI * pow(pow(dot(normal, halfway), 2.0) * (roughness2 - 1) + 1, 2.0));

    let k = pow(roughness2 + 1.0, 2.0) / 8.0;

    let normal_dot_light = dot(normal, light_dir);
    let normal_dot_view = dot(normal, view_dir);
    let geometry = (normal_dot_light / (normal_dot_light * (1-k)+k))
        * (normal_dot_view / (normal_dot_view * (1-k)+k));

    let specular = (fresnel * distribution * geometry)
        / (4 * dot(light_dir, halfway) * dot(view_dir, halfway));
    return k_d * diffuse + k_s * specular;
}

fn tone_map(hdr: vec3f) -> vec3f {
    let m1 = mat3x3(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let m2 = mat3x3(
        1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    let v = m1 * hdr;
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return clamp(m2 * (a / b), vec3(0.0), vec3(1.0));
}

fn into_vec3_pos(pos: vec4f) -> vec3f {
    return pos.xyz / pos.w;
}

fn ndc_to_uv(coord: vec2f) -> vec2f {
    return vec2f(fma(coord.x, 0.5, 0.5), fma(coord.y, -0.5, 0.5));
}