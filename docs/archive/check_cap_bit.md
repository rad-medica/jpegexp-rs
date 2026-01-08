# HTJ2K CAP Marker Bit Investigation

From our tests:
- Our encoder sets: Pcap = 0x00004000 (bit 14)
- OpenHTJ2K sets: Pcap = 0x00020000 (bit 17)

Binary breakdown:
- Bit 14: 0x00004000 = 0b00000000_00000000_01000000_00000000
- Bit 17: 0x00020000 = 0b00000000_00000010_00000000_00000000

Need to verify correct bit from ISO/IEC 15444-15 specification.
