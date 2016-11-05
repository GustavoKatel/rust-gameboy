use mem::GBMem;
use cpu::GBCpu;
use sdl_display::SDLDisplay;

// References:
// - http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-GPU-Timings
// - http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-Graphics

enum GBGpuMode {
    HBLANK,
    VBLANK,
    OAM,
    VRAM,
}

pub struct GBGpu {

    mode: GBGpuMode,
    cycles: usize,
    drawing_line: usize,

}

impl GBGpu {

    pub fn new() -> GBGpu {
        GBGpu{
            mode: GBGpuMode::HBLANK,
            cycles: 0,
            drawing_line: 0,
        }
    }

    pub fn step(&mut self, cpu: &mut GBCpu, display: &mut SDLDisplay) {

        self.cycles += cpu.get_last_op_cycles();

        match self.mode {
            // TODO: check 0xff41 (stat flags)
            // just rendered a line, going back to the left side of the screen
            GBGpuMode::HBLANK => {
                // HBLANK duration in cycles: 204
                if self.cycles >= 204 {

                    self.cycles = 0;
                    self.drawing_line += 1;
                    // LY mem register. It stores the current line
                    cpu.set_memreg_ly(self.drawing_line as u8);

                    // check if we reached the last line. If so, enter vblank and draw the frame
                    if self.drawing_line == 143 {
                        // Enter vblank
                        // is VBLANK interrupt enabled?
                        if cpu.is_interrupt_enabled(0) {
                            // request vblank interrupt
                            cpu.set_interrupt_request(0, true);
                        }
                        self.mode = GBGpuMode::VBLANK;
                        // TODO: draw the frame. SDL(?) id:7
                    } else {
                        // just one more line, start reading the sprites
                        self.mode = GBGpuMode::OAM;
                    }

                }

            },
            GBGpuMode::VBLANK => {
                // VBLANK duration: 456 * 10 lines
                if self.cycles >= 456 {
                    self.cycles = 0;
                    self.drawing_line += 1;
                    // LY mem register. It stores the current line
                    cpu.set_memreg_ly(self.drawing_line as u8);

                    if self.drawing_line > 153 {
                        // Restart scanning modes
                        self.mode = GBGpuMode::OAM;
                        self.drawing_line = 0;
                        // LY mem register. It stores the current line
                        cpu.set_memreg_ly(self.drawing_line as u8);
                    }
                }
            },
            GBGpuMode::OAM => {
                // loop for a while in the OAM mode and go to the VRAM mode after
                if self.cycles >= 80 {
                    self.cycles = 0;
                    self.mode = GBGpuMode::VRAM;
                }
            },
            GBGpuMode::VRAM => {
                // loop for a while in the OAM mode and then writes the new line to the buffer
                if self.cycles >= 172 {
                    self.cycles = 0;
                    self.mode = GBGpuMode::HBLANK;

                    // Write a scanline to the framebuffer
                    // TODO: draw line id:8
                }
            },
        }

    }

}
