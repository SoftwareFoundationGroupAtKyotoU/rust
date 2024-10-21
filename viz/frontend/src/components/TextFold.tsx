export const TextFold: React.FC<{ text: string; maxLength: number }> = ({
    text: message,
    maxLength,
}) => (
    <span className="[&:hover>.preview]:hidden [&:hover>.full]:inline break-all">
        <span className="preview">
            {/* preview */}
            {message.slice(0, maxLength)}
            {message.length > maxLength ? (
                <span className="border border-current opacity-30">...</span>
            ) : null}
        </span>
        <span className="full hidden whitespace-pre-wrap">
            {/* content */}
            {message}
        </span>
    </span>
);
