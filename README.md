# Physically based rendering

This project aims to implement rasterizing based physically based rendering.

Currently, it can load glTF (.gltf/.glb) file and render it on the screen. The color is determined by local coordinate.
You can create glTF file using `export` feature in Blender. You should turn on `+Y up`, `cameras`, and `punctual lights` when exporting. The scene should have at least one camera.

Currently, it only support global material. You can change this global material with keyboard.

## Running

```
cargo run
```


## Controls

    WASD : move
    Space/LShift : change height
    +/- : change camera FOV
    U/J : change material roughness
    I/K : change material metallic
    O/L : change material hue
    Escape : exit
