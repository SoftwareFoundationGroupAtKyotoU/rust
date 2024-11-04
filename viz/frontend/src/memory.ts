export type MiriMemoryKind =
    | "Rust"
    | "Miri"
    | "C"
    | "WinHeap"
    | "WinLocal"
    | "Machine"
    | "Runtime"
    | "Global"
    | "ExternStatic"
    | "Tls"
    | "Mmap";

export type MemoryKind =
    | "Stack"
    | "CallerLocation"
    | `Machine(${MiriMemoryKind})`;

export const unleakableMemoryKinds: MemoryKind[] = [
    "Machine(Machine)",
    "Machine(Global)",
    "Machine(ExternStatic)",
    "Machine(Tls)",
    // TODO: confirm how to handle this
    "Machine(Runtime)",
];

export const isMemoryKindLeakable = (kind: MemoryKind): boolean =>
    !unleakableMemoryKinds.includes(kind);

export type LeakedState = "Unleakable" | "Leaked" | "Reachable";

export const computeLeakedState = (
    kind: MemoryKind,
    reachable: boolean
): LeakedState =>
    !isMemoryKindLeakable(kind)
        ? "Unleakable"
        : reachable
        ? "Reachable"
        : "Leaked";
