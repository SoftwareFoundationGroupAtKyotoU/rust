import clsx from "clsx";
import { useState } from "react";

export const TextFold: React.FC<{ text: string; maxLength: number }> = ({
    text: message,
    maxLength,
}) => {
    const [mustShow, setMustShow] = useState(false);
    return (
        <span
            onClick={() => setMustShow((m) => !m)}
            className={clsx("break-all cursor-pointer", {
                "[&:hover>.preview]:hidden [&:hover>.full]:inline": !mustShow,
                "[&>.preview]:hidden [&>.full]:inline": mustShow,
            })}
        >
            <span className="preview">
                {/* preview */}
                {message.slice(0, maxLength)}
                {message.length > maxLength ? (
                    <span className="border border-current opacity-30">
                        ...
                    </span>
                ) : null}
            </span>
            <span className="full hidden whitespace-pre-wrap">
                {/* content */}
                {message}
            </span>
        </span>
    );
};
