import { useMemo } from "react";
import { chunk } from "../utils";

type BytesDisplayProps = {
    bytes: number[];
    width?: number;
};

const formatByte = (byte: number) =>
    byte.toString(16).padStart(2, "0").toUpperCase();

export const BytesDisplay: React.FC<BytesDisplayProps> = ({ bytes, width }) => {
    const formattedPreview = useMemo(
        () =>
            bytes.slice(0, 16).map(formatByte).join(" ") +
            (bytes.length > 16 ? " .." : ""),
        [bytes, width]
    );

    const formattedChunk = useMemo(
        () =>
            chunk(bytes, width ?? 16)
                .map((chunk) => chunk.map(formatByte).join(" "))
                .join("\n"),
        [bytes, width]
    );

    return (
        <div className="inline-block [&:hover>.preview]:hidden [&:hover>.full]:inline-block">
            <pre className="preview inline-block">{formattedPreview}</pre>
            <pre className="full hidden">{formattedChunk}</pre>
        </div>
    );
};
