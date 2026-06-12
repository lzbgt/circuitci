# Peer `urine_monitor` STM32 Toolchain Evidence

This note records local evidence from the peer repository
`../urine_monitor`. It corrects the earlier assumption that no ARM embedded
compiler is available on the host.

## Build Scripts

The peer repository documents STM32 firmware builds in `AGENTS.md` and
`README.md`:

- STM32L431 node: `./tools/build_stm32l431_node.sh`
- STM32L431 resident bootloader: `./tools/build_stm32l431_bootloader.sh`

Both scripts use CMake and the shared toolchain file:

```text
firmware_stm32l431_node/cmake/stm32cubeide_gcc_toolchain.cmake
```

The node script supports the fabricated UM board:

```text
./tools/build_stm32l431_node.sh --board um-stm32l4-v1
```

The script configures the delivered UM-board options, including fast boot,
CH340C/USART1 service path preservation, and the board-specific LoRa software
SPI compensation.

## Compiler Discovery

The compiler is not globally visible as `arm-none-eabi-gcc` on `PATH`. The
peer toolchain file discovers GNU Arm tools inside STM32CubeIDE:

```text
/Applications/STM32CubeIDE.app/Contents/Eclipse/plugins/com.st.stm32cube.ide.mcu.externaltools.gnu-tools-for-stm32.*.macos64_*/tools/bin
```

Local inspection found:

```text
/Applications/STM32CubeIDE.app/Contents/Eclipse/plugins/com.st.stm32cube.ide.mcu.externaltools.gnu-tools-for-stm32.13.3.rel1.macos64_1.0.0.202411102158/tools/bin/arm-none-eabi-gcc
```

Compiler version:

```text
arm-none-eabi-gcc (GNU Tools for STM32 13.3.rel1.20240926-1715) 13.3.1 20240614
```

## Build Verification

Both peer STM32 build scripts were run successfully from `../urine_monitor`:

```text
./tools/build_stm32l431_node.sh --board um-stm32l4-v1
Built: firmware_stm32l431_node/build/stm32l431_node.elf

./tools/build_stm32l431_bootloader.sh
Built: firmware_stm32l431_bootloader/build/stm32l431_bootloader.elf
```

Observed build outputs:

```text
firmware_stm32l431_node/build/stm32l431_node.elf
firmware_stm32l431_node/build/stm32l431_node.bin
firmware_stm32l431_bootloader/build/stm32l431_bootloader.elf
firmware_stm32l431_bootloader/build/stm32l431_bootloader.bin
```

## CircuitCI Implication

CircuitCI firmware-in-loop scenarios should not assume MCU compilers are on
`PATH`. They should be able to call explicit repo-local build scripts and then
consume declared ELF/BIN artifacts. The `firmware.build` block exists for that
purpose and records `firmware_build.log` plus declared build outputs as report
artifacts.
