
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
    pos: vec3f,
    typ: u32,
    color: vec3f,
    intensity: f32,
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

@group(0) @binding(3)
var<uniform> material: vec3f;

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    var color = vec3f(0.0);
    for(var i = 0; i < 4; i++) {
        let light = lights[i];
        switch light.typ {
            case 1: {
                let normal = in.normal;
                let light_in = in.world_pos - light.pos;
                let light_out = camera_pos - in.world_pos;

                let light_in_n = normalize(-light_in);
                let light_out_n = normalize(light_out);

                let halfway = normalize(light_in_n + light_out_n);

                let light_distance = length(light_in);
                let light_power = 0.01 * light.intensity / (light_distance * light_distance);
                
                let roughness = material.x;
                let metallic = material.y;
                let hue = material.z;
                let hueX = 1.0 - abs((hue * 6.0) % 2.0 - 1.0);
                var albedo = vec3f(1.0);
                switch(u32(hue * 6.0)) {
                    case 0: {
                        albedo = vec3f(1.0, hueX, 0.0);
                    }
                    case 1: {
                        albedo = vec3f(hueX, 1.0, 0.0);
                    }
                    case 2: {
                        albedo = vec3f(0.0, 1.0, hueX);
                    }
                    case 3: {
                        albedo = vec3f(0.0, hueX, 1.0);
                    }
                    case 4: {
                        albedo = vec3f(hueX, 0.0, 1.0);
                    }
                    case 5: {
                        albedo = vec3f(1.0, 0.0, hueX);
                    }
                    default: {

                    }
                }

                let f_0 = mix(vec3f(0.4), albedo, metallic);
                let fresnel = f_0 + (1 - f_0) * pow((1 - dot(halfway, light_out_n)), 5.0);

                let k_s = fresnel;
                let k_d = (vec3f(1.0) - k_s) * (1.0 - metallic);
                let diffuse = albedo / PI;

                let roughness2 = roughness * roughness;
                let distribution = roughness2 / (PI * pow(pow(dot(normal, halfway), 2.0) * (roughness2 - 1) + 1, 2.0));

                let k = pow(roughness2 + 1.0, 2.0) / 8.0;

                let normal_dot_in = dot(normal, light_in_n);
                let normal_dot_out = dot(normal, light_out_n);
                let geometry = (normal_dot_in / (normal_dot_in * (1-k)+k)) * (normal_dot_out / (normal_dot_out * (1-k)+k));

                let specular = (fresnel * distribution * geometry) / (4 * dot(light_in_n, halfway) * dot(light_out_n, halfway));

                let brdf = k_d * diffuse + k_s * specular;

                color += brdf * light_power * max(normal_dot_in, 0.0);
            }
            default: {

            }
        }
    }

    return vec4f(color, 1.0);
}