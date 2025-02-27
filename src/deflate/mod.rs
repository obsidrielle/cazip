mod encode;

#[derive(Debug, Clone, Copy)]
enum BlockType {
    Raw = 0b00,
    Fixed = 0b01,
    Dynamic = 0b10,
}