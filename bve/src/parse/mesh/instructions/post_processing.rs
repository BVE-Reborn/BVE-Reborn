use crate::parse::{
    mesh::{instructions::*, MeshError, MeshErrorKind},
    Span,
};
use glam::{f32::Vec3A, Vec2};
use log::trace;
use std::f32::consts::PI;

/// Prepares instructions for execution.
///
/// Performs two postprocessing steps on the instruction list:
///
/// - Splits up [`Cube`] and [`Cylinder`] into their respective [`AddVertex`] and [`AddFace`] instructions. This allows
///   the following step to happen.
/// - Applies all [`SetTextureCoordinates`] to their vertex, moving the data into the [`AddVertex`] data structure.
///
/// The last bit of post processing must be done as the executor isn't actually able to edit the resulting
/// [`Vertex`](crate::load::mesh::Vertex) structs arbitrarily by index as [`SetTextureCoordinates`] requires.
///
/// Errors are taken from [`InstructionList::errors`] and any new ones encountered are appended and put in the result's
/// [`InstructionList::errors`]. These errors are all non-fatal, so [`Result`] can't be used.
#[must_use]
pub fn post_process(mut instructions: InstructionList) -> InstructionList {
    trace!("Post processing mesh");

    let mut output = Vec::new();
    let meshes = instructions
        .instructions
        .split(|i| i.data == InstructionData::CreateMeshBuilder(CreateMeshBuilder));
    for mesh in meshes {
        let mesh = process_compound(mesh);
        let mesh = merge_texture_coords(&mesh, &mut instructions.errors);
        output.push(Instruction {
            span: Span::none(),
            data: InstructionData::CreateMeshBuilder(CreateMeshBuilder),
        });
        output.extend(mesh);
    }

    instructions.instructions = output;

    instructions
}

/// Creates a `AddVertex` instruction from a position.
fn create_vertex(original: &Instruction, position: Vec3A) -> Instruction {
    Instruction {
        span: original.span,
        data: InstructionData::AddVertex(AddVertex {
            position,
            normal: Vec3A::zero(),
            texture_coord: Vec2::zero(),
        }),
    }
}

/// Creates `AddFace` instruction from an index list.
const fn create_face(original: &Instruction, indexes: Vec<usize>) -> Instruction {
    Instruction {
        span: original.span,
        data: InstructionData::AddFace(AddFace {
            indexes,
            sides: Sides::One,
        }),
    }
}

/// For each the mesh given, replaces `Cube` and `Cylinder` commands with the appropriate `AddVertex` and `AddFace`
/// commands.
#[allow(clippy::identity_op)]
fn process_compound(mesh: &[Instruction]) -> Vec<Instruction> {
    let mut result = Vec::new();

    // Need to keep track of the current vertex index so cubes and cylinders can use the correct indices
    let mut vertex_index = 0;
    for instruction in mesh {
        match &instruction.data {
            InstructionData::AddVertex(..) => {
                result.push(instruction.clone());
                vertex_index += 1;
            }
            InstructionData::Cube(cube) => {
                // http://openbve-project.net/documentation/HTML/object_cubecylinder.html

                let x = cube.half_dim.x();
                let y = cube.half_dim.y();
                let z = cube.half_dim.z();

                result.push(create_vertex(instruction, Vec3A::new(x, y, -z)));
                result.push(create_vertex(instruction, Vec3A::new(x, -y, -z)));
                result.push(create_vertex(instruction, Vec3A::new(-x, -y, -z)));
                result.push(create_vertex(instruction, Vec3A::new(-x, y, -z)));
                result.push(create_vertex(instruction, Vec3A::new(x, y, z)));
                result.push(create_vertex(instruction, Vec3A::new(x, -y, z)));
                result.push(create_vertex(instruction, Vec3A::new(-x, -y, z)));
                result.push(create_vertex(instruction, Vec3A::new(-x, y, z)));

                let vi = vertex_index;

                result.push(create_face(instruction, vec![vi + 0, vi + 1, vi + 2, vi + 3]));
                result.push(create_face(instruction, vec![vi + 0, vi + 4, vi + 5, vi + 1]));
                result.push(create_face(instruction, vec![vi + 0, vi + 3, vi + 7, vi + 4]));
                result.push(create_face(instruction, vec![vi + 6, vi + 5, vi + 4, vi + 7]));
                result.push(create_face(instruction, vec![vi + 6, vi + 7, vi + 3, vi + 2]));
                result.push(create_face(instruction, vec![vi + 6, vi + 2, vi + 1, vi + 5]));

                vertex_index += 8;
            }
            InstructionData::Cylinder(cylinder) => {
                // http://openbve-project.net/documentation/HTML/object_cubecylinder.html

                // Convert args to format used in above documentation
                let n = cylinder.sides;
                let n_f32 = n as f32;
                let r1 = cylinder.upper_radius;
                let r2 = cylinder.lower_radius;
                let h = cylinder.height;

                // Vertices
                for i in (0..n).map(|i| i as f32) {
                    let trig_arg = (2.0 * PI * i) / n_f32;
                    let cos = trig_arg.cos();
                    let sin = trig_arg.sin();
                    result.push(create_vertex(instruction, Vec3A::new(cos * r1, h / 2.0, sin * r1)));
                    result.push(create_vertex(instruction, Vec3A::new(cos * r2, -h / 2.0, sin * r2)));
                }

                // Faces
                let vi = vertex_index;

                let split = n.saturating_sub(1) as usize;
                for i in 0..split {
                    result.push(create_face(instruction, vec![
                        vi + (2 * i + 2),
                        vi + (2 * i + 3),
                        vi + (2 * i + 1),
                        vi + (2 * i + 0),
                    ]));
                    result.push(create_face(instruction, vec![
                        vi + 0,
                        vi + 1,
                        vi + (2 * i + 1),
                        vi + (2 * i + 0),
                    ]));
                }

                vertex_index += (2 * n) as usize;
            }
            _ => {
                result.push(instruction.clone());
            }
        }
    }

    result
}

/// For each mesh give, fold the `SetTextureCoordinates` into the `AddVertex` commands
fn merge_texture_coords(mesh: &[Instruction], errors: &mut Vec<MeshError>) -> Vec<Instruction> {
    let mut result = Vec::new();
    // The instruction where the vertex index n is created is at result[vertex_indices[n]]
    let mut vertex_indices = Vec::new();

    for instruction in mesh {
        match &instruction.data {
            InstructionData::AddVertex(..) => {
                // Add the index for this vertex so it can be found again
                vertex_indices.push(result.len());
                result.push(instruction.clone());
            }
            InstructionData::SetTextureCoordinates(data) => {
                // Issue error if the index is out of range
                if data.index >= vertex_indices.len() {
                    errors.push(MeshError {
                        location: instruction.span,
                        kind: MeshErrorKind::OutOfBounds { idx: data.index },
                    });
                    continue;
                }
                // Go and set the texture coord of the AddVertex command
                // Unless there's a bug in the code, this is guaranteed to be a AddVertex.
                match &mut result[vertex_indices[data.index]].data {
                    InstructionData::AddVertex(vert) => {
                        vert.texture_coord = data.coords;
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                result.push(instruction.clone());
            }
        }
    }

    result
}
