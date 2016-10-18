
use mem::GBMem;

pub struct GBCpu {
    sp  : u16, // stack pointer
    pc  : u16, // program counter
    mem : GBMem, // ram
    AF: u16, // reg 16bits: A (8bits) and flags (F) b'Zero-N(subtract)-HalfCarry-CarryFlag-0000
    BC: u16, // reg 16bits
    DE: u16, // reg 16bits
    HL: u16, // reg 16bits
    stop_flag: bool, // stop flag used by the stop instruction
}

enum GBData<'a> {
    D16(u16),
    D8(u8),
    ADDRESS(usize),
    OTHER(&'a str),
}

impl GBCpu {

    pub fn new(mem: GBMem) -> GBCpu {
        GBCpu {
            sp: 0x0,
            pc: 0x0,
            mem: mem,
            AF: 0x0,
            BC: 0x0,
            DE: 0x0,
            HL: 0x0,
            stop_flag: false,
        }
    }

    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn get_mem_ref<'a> (&'a self) -> &'a GBMem {
        &self.mem
    }

    pub fn step(&mut self) {
        self.exec_next_op();
    }

    fn data_p<'a>(&'a mut self, arg: &'a str) -> GBData {
        GBData::OTHER(arg)
    }

    fn op_adc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_add<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_and<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_call<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_ccf(&mut self) {

    }

    fn op_cpl(&mut self) {

    }

    fn op_cp<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_daa(&mut self) {

    }

    fn op_dec<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_di(&mut self) {

    }

    fn op_ei(&mut self) {

    }

    fn op_halt(&mut self) {

    }

    fn op_inc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_jp<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_jr<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_ldh<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_ld<'a> (&mut self, args: &'a Vec<&'a str>) {

        println!("ld", );

        for i in args.iter() {
            println!("arg: {:?}", i);
        }

        self.pc += 1;

    }

    fn op_none(&mut self) {

    }

    fn op_nop(&mut self) {

    }

    fn op_or<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_pop<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_prefix<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_push<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_reti(&mut self) {

    }

    fn op_ret<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_rla(&mut self) {

    }

    fn op_rlca(&mut self) {

    }

    fn op_rra(&mut self) {

    }

    fn op_rrca(&mut self) {

    }

    fn op_rst<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sbc<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_scf(&mut self) {

    }

    fn op_stop<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_sub<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn op_xor<'a> (&mut self, args: &'a Vec<&'a str>) {

    }

    fn exec_next_op(&mut self) {

        let byte = self.mem.get(self.pc as usize);

        match byte & 0xFF {

            0x0 => self.op_nop(),
            0x1 => self.op_ld(&vec!("BC","d16")),
            0x2 => self.op_ld(&vec!("(BC)","A")),
            0x3 => self.op_inc(&vec!("BC")),
            0x4 => self.op_inc(&vec!("B")),
            0x5 => self.op_dec(&vec!("B")),
            0x6 => self.op_ld(&vec!("B","d8")),
            0x7 => self.op_rlca(),
            0x8 => self.op_ld(&vec!("(a16)","SP")),
            0x9 => self.op_add(&vec!("HL","BC")),
            0xa => self.op_ld(&vec!("A","(BC)")),
            0xb => self.op_dec(&vec!("BC")),
            0xc => self.op_inc(&vec!("C")),
            0xd => self.op_dec(&vec!("C")),
            0xe => self.op_ld(&vec!("C","d8")),
            0xf => self.op_rrca(),
            0x10 => self.op_stop(&vec!("0")),
            0x11 => self.op_ld(&vec!("DE","d16")),
            0x12 => self.op_ld(&vec!("(DE)","A")),
            0x13 => self.op_inc(&vec!("DE")),
            0x14 => self.op_inc(&vec!("D")),
            0x15 => self.op_dec(&vec!("D")),
            0x16 => self.op_ld(&vec!("D","d8")),
            0x17 => self.op_rla(),
            0x18 => self.op_jr(&vec!("r8")),
            0x19 => self.op_add(&vec!("HL","DE")),
            0x1a => self.op_ld(&vec!("A","(DE)")),
            0x1b => self.op_dec(&vec!("DE")),
            0x1c => self.op_inc(&vec!("E")),
            0x1d => self.op_dec(&vec!("E")),
            0x1e => self.op_ld(&vec!("E","d8")),
            0x1f => self.op_rra(),
            0x20 => self.op_jr(&vec!("NZ","r8")),
            0x21 => self.op_ld(&vec!("HL","d16")),
            0x22 => self.op_ld(&vec!("(HL+)","A")),
            0x23 => self.op_inc(&vec!("HL")),
            0x24 => self.op_inc(&vec!("H")),
            0x25 => self.op_dec(&vec!("H")),
            0x26 => self.op_ld(&vec!("H","d8")),
            0x27 => self.op_daa(),
            0x28 => self.op_jr(&vec!("Z","r8")),
            0x29 => self.op_add(&vec!("HL","HL")),
            0x2a => self.op_ld(&vec!("A","(HL+)")),
            0x2b => self.op_dec(&vec!("HL")),
            0x2c => self.op_inc(&vec!("L")),
            0x2d => self.op_dec(&vec!("L")),
            0x2e => self.op_ld(&vec!("L","d8")),
            0x2f => self.op_cpl(),
            0x30 => self.op_jr(&vec!("NC","r8")),
            0x31 => self.op_ld(&vec!("SP","d16")),
            0x32 => self.op_ld(&vec!("(HL-)","A")),
            0x33 => self.op_inc(&vec!("SP")),
            0x34 => self.op_inc(&vec!("(HL)")),
            0x35 => self.op_dec(&vec!("(HL)")),
            0x36 => self.op_ld(&vec!("(HL)","d8")),
            0x37 => self.op_scf(),
            0x38 => self.op_jr(&vec!("C","r8")),
            0x39 => self.op_add(&vec!("HL","SP")),
            0x3a => self.op_ld(&vec!("A","(HL-)")),
            0x3b => self.op_dec(&vec!("SP")),
            0x3c => self.op_inc(&vec!("A")),
            0x3d => self.op_dec(&vec!("A")),
            0x3e => self.op_ld(&vec!("A","d8")),
            0x3f => self.op_ccf(),
            0x40 => self.op_ld(&vec!("B","B")),
            0x41 => self.op_ld(&vec!("B","C")),
            0x42 => self.op_ld(&vec!("B","D")),
            0x43 => self.op_ld(&vec!("B","E")),
            0x44 => self.op_ld(&vec!("B","H")),
            0x45 => self.op_ld(&vec!("B","L")),
            0x46 => self.op_ld(&vec!("B","(HL)")),
            0x47 => self.op_ld(&vec!("B","A")),
            0x48 => self.op_ld(&vec!("C","B")),
            0x49 => self.op_ld(&vec!("C","C")),
            0x4a => self.op_ld(&vec!("C","D")),
            0x4b => self.op_ld(&vec!("C","E")),
            0x4c => self.op_ld(&vec!("C","H")),
            0x4d => self.op_ld(&vec!("C","L")),
            0x4e => self.op_ld(&vec!("C","(HL)")),
            0x4f => self.op_ld(&vec!("C","A")),
            0x50 => self.op_ld(&vec!("D","B")),
            0x51 => self.op_ld(&vec!("D","C")),
            0x52 => self.op_ld(&vec!("D","D")),
            0x53 => self.op_ld(&vec!("D","E")),
            0x54 => self.op_ld(&vec!("D","H")),
            0x55 => self.op_ld(&vec!("D","L")),
            0x56 => self.op_ld(&vec!("D","(HL)")),
            0x57 => self.op_ld(&vec!("D","A")),
            0x58 => self.op_ld(&vec!("E","B")),
            0x59 => self.op_ld(&vec!("E","C")),
            0x5a => self.op_ld(&vec!("E","D")),
            0x5b => self.op_ld(&vec!("E","E")),
            0x5c => self.op_ld(&vec!("E","H")),
            0x5d => self.op_ld(&vec!("E","L")),
            0x5e => self.op_ld(&vec!("E","(HL)")),
            0x5f => self.op_ld(&vec!("E","A")),
            0x60 => self.op_ld(&vec!("H","B")),
            0x61 => self.op_ld(&vec!("H","C")),
            0x62 => self.op_ld(&vec!("H","D")),
            0x63 => self.op_ld(&vec!("H","E")),
            0x64 => self.op_ld(&vec!("H","H")),
            0x65 => self.op_ld(&vec!("H","L")),
            0x66 => self.op_ld(&vec!("H","(HL)")),
            0x67 => self.op_ld(&vec!("H","A")),
            0x68 => self.op_ld(&vec!("L","B")),
            0x69 => self.op_ld(&vec!("L","C")),
            0x6a => self.op_ld(&vec!("L","D")),
            0x6b => self.op_ld(&vec!("L","E")),
            0x6c => self.op_ld(&vec!("L","H")),
            0x6d => self.op_ld(&vec!("L","L")),
            0x6e => self.op_ld(&vec!("L","(HL)")),
            0x6f => self.op_ld(&vec!("L","A")),
            0x70 => self.op_ld(&vec!("(HL)","B")),
            0x71 => self.op_ld(&vec!("(HL)","C")),
            0x72 => self.op_ld(&vec!("(HL)","D")),
            0x73 => self.op_ld(&vec!("(HL)","E")),
            0x74 => self.op_ld(&vec!("(HL)","H")),
            0x75 => self.op_ld(&vec!("(HL)","L")),
            0x76 => self.op_halt(),
            0x77 => self.op_ld(&vec!("(HL)","A")),
            0x78 => self.op_ld(&vec!("A","B")),
            0x79 => self.op_ld(&vec!("A","C")),
            0x7a => self.op_ld(&vec!("A","D")),
            0x7b => self.op_ld(&vec!("A","E")),
            0x7c => self.op_ld(&vec!("A","H")),
            0x7d => self.op_ld(&vec!("A","L")),
            0x7e => self.op_ld(&vec!("A","(HL)")),
            0x7f => self.op_ld(&vec!("A","A")),
            0x80 => self.op_add(&vec!("A","B")),
            0x81 => self.op_add(&vec!("A","C")),
            0x82 => self.op_add(&vec!("A","D")),
            0x83 => self.op_add(&vec!("A","E")),
            0x84 => self.op_add(&vec!("A","H")),
            0x85 => self.op_add(&vec!("A","L")),
            0x86 => self.op_add(&vec!("A","(HL)")),
            0x87 => self.op_add(&vec!("A","A")),
            0x88 => self.op_adc(&vec!("A","B")),
            0x89 => self.op_adc(&vec!("A","C")),
            0x8a => self.op_adc(&vec!("A","D")),
            0x8b => self.op_adc(&vec!("A","E")),
            0x8c => self.op_adc(&vec!("A","H")),
            0x8d => self.op_adc(&vec!("A","L")),
            0x8e => self.op_adc(&vec!("A","(HL)")),
            0x8f => self.op_adc(&vec!("A","A")),
            0x90 => self.op_sub(&vec!("B")),
            0x91 => self.op_sub(&vec!("C")),
            0x92 => self.op_sub(&vec!("D")),
            0x93 => self.op_sub(&vec!("E")),
            0x94 => self.op_sub(&vec!("H")),
            0x95 => self.op_sub(&vec!("L")),
            0x96 => self.op_sub(&vec!("(HL)")),
            0x97 => self.op_sub(&vec!("A")),
            0x98 => self.op_sbc(&vec!("A","B")),
            0x99 => self.op_sbc(&vec!("A","C")),
            0x9a => self.op_sbc(&vec!("A","D")),
            0x9b => self.op_sbc(&vec!("A","E")),
            0x9c => self.op_sbc(&vec!("A","H")),
            0x9d => self.op_sbc(&vec!("A","L")),
            0x9e => self.op_sbc(&vec!("A","(HL)")),
            0x9f => self.op_sbc(&vec!("A","A")),
            0xa0 => self.op_and(&vec!("B")),
            0xa1 => self.op_and(&vec!("C")),
            0xa2 => self.op_and(&vec!("D")),
            0xa3 => self.op_and(&vec!("E")),
            0xa4 => self.op_and(&vec!("H")),
            0xa5 => self.op_and(&vec!("L")),
            0xa6 => self.op_and(&vec!("(HL)")),
            0xa7 => self.op_and(&vec!("A")),
            0xa8 => self.op_xor(&vec!("B")),
            0xa9 => self.op_xor(&vec!("C")),
            0xaa => self.op_xor(&vec!("D")),
            0xab => self.op_xor(&vec!("E")),
            0xac => self.op_xor(&vec!("H")),
            0xad => self.op_xor(&vec!("L")),
            0xae => self.op_xor(&vec!("(HL)")),
            0xaf => self.op_xor(&vec!("A")),
            0xb0 => self.op_or(&vec!("B")),
            0xb1 => self.op_or(&vec!("C")),
            0xb2 => self.op_or(&vec!("D")),
            0xb3 => self.op_or(&vec!("E")),
            0xb4 => self.op_or(&vec!("H")),
            0xb5 => self.op_or(&vec!("L")),
            0xb6 => self.op_or(&vec!("(HL)")),
            0xb7 => self.op_or(&vec!("A")),
            0xb8 => self.op_cp(&vec!("B")),
            0xb9 => self.op_cp(&vec!("C")),
            0xba => self.op_cp(&vec!("D")),
            0xbb => self.op_cp(&vec!("E")),
            0xbc => self.op_cp(&vec!("H")),
            0xbd => self.op_cp(&vec!("L")),
            0xbe => self.op_cp(&vec!("(HL)")),
            0xbf => self.op_cp(&vec!("A")),
            0xc0 => self.op_ret(&vec!("NZ")),
            0xc1 => self.op_pop(&vec!("BC")),
            0xc2 => self.op_jp(&vec!("NZ","a16")),
            0xc3 => self.op_jp(&vec!("a16")),
            0xc4 => self.op_call(&vec!("NZ","a16")),
            0xc5 => self.op_push(&vec!("BC")),
            0xc6 => self.op_add(&vec!("A","d8")),
            0xc7 => self.op_rst(&vec!("00H")),
            0xc8 => self.op_ret(&vec!("Z")),
            0xc9 => self.op_ret(&vec!()),
            0xca => self.op_jp(&vec!("Z","a16")),
            0xcb => self.op_prefix(&vec!("CB")),
            0xcc => self.op_call(&vec!("Z","a16")),
            0xcd => self.op_call(&vec!("a16")),
            0xce => self.op_adc(&vec!("A","d8")),
            0xcf => self.op_rst(&vec!("08H")),
            0xd0 => self.op_ret(&vec!("NC")),
            0xd1 => self.op_pop(&vec!("DE")),
            0xd2 => self.op_jp(&vec!("NC","a16")),
            0xd3 => self.op_none(),
            0xd4 => self.op_call(&vec!("NC","a16")),
            0xd5 => self.op_push(&vec!("DE")),
            0xd6 => self.op_sub(&vec!("d8")),
            0xd7 => self.op_rst(&vec!("10H")),
            0xd8 => self.op_ret(&vec!("C")),
            0xd9 => self.op_reti(),
            0xda => self.op_jp(&vec!("C","a16")),
            0xdb => self.op_none(),
            0xdc => self.op_call(&vec!("C","a16")),
            0xdd => self.op_none(),
            0xde => self.op_sbc(&vec!("A","d8")),
            0xdf => self.op_rst(&vec!("18H")),
            0xe0 => self.op_ldh(&vec!("(a8)","A")),
            0xe1 => self.op_pop(&vec!("HL")),
            0xe2 => self.op_ld(&vec!("(C)","A")),
            0xe3 => self.op_none(),
            0xe4 => self.op_none(),
            0xe5 => self.op_push(&vec!("HL")),
            0xe6 => self.op_and(&vec!("d8")),
            0xe7 => self.op_rst(&vec!("20H")),
            0xe8 => self.op_add(&vec!("SP","r8")),
            0xe9 => self.op_jp(&vec!("(HL)")),
            0xea => self.op_ld(&vec!("(a16)","A")),
            0xeb => self.op_none(),
            0xec => self.op_none(),
            0xed => self.op_none(),
            0xee => self.op_xor(&vec!("d8")),
            0xef => self.op_rst(&vec!("28H")),
            0xf0 => self.op_ldh(&vec!("A","(a8)")),
            0xf1 => self.op_pop(&vec!("AF")),
            0xf2 => self.op_ld(&vec!("A","(C)")),
            0xf3 => self.op_di(),
            0xf4 => self.op_none(),
            0xf5 => self.op_push(&vec!("AF")),
            0xf6 => self.op_or(&vec!("d8")),
            0xf7 => self.op_rst(&vec!("30H")),
            0xf8 => self.op_ld(&vec!("HL","SP+r8")),
            0xf9 => self.op_ld(&vec!("SP","HL")),
            0xfa => self.op_ld(&vec!("A","(a16)")),
            0xfb => self.op_ei(),
            0xfc => self.op_none(),
            0xfd => self.op_none(),
            0xfe => self.op_cp(&vec!("d8")),
            0xff => self.op_rst(&vec!("38H")),
            _ => panic!("Unknown OP"),

        }

    }

}
