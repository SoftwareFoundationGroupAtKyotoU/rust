import React, { useState } from "react";

type FoldableProps = {
    header: React.ReactNode;
    defaultFolded?: boolean;
};

export const Foldable: React.FC<React.PropsWithChildren<FoldableProps>> = ({
    header,
    children,
    defaultFolded,
}) => {
    const [isFolded, setIsFolded] = useState(defaultFolded ?? false);

    return (
        <div
            className="leading-[1.2rem]"
            style={{ fontFamily: "Consolas, 'Courier New', monospace" }}
        >
            <div
                className="cursor-pointer flex gap-2"
                onClick={() => setIsFolded((f) => !f)}
            >
                <div>{isFolded ? "(+)" : "(-)"}</div>
                <div>{header}</div>
            </div>
            <div
                className="ml-[4ch]"
                style={{
                    display: isFolded ? "none" : "block",
                }}
            >
                {children}
            </div>
        </div>
    );
};
