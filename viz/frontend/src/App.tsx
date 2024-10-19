import React, { useCallback, useMemo, useRef, useState } from "react";
import useSWR from "swr";

function readFileToString(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => resolve(event.target?.result as string);
        reader.onerror = (error) => reject(error);
        reader.readAsText(file);
    });
}

type Node = {
    alloc_id?: number;
    ty: string;
    offset: number;
    info_messages: string[];
    error_messages: string[];
    children: Node[];
};

const TextFold: React.FC<{ text: string; maxLength: number }> = ({
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

const Visualizer: React.FC<{ node: Node }> = ({ node }) => {
    const [isFolded, setIsFolded] = useState(false);
    return (
        <div
            className="leading-[1.2rem]"
            style={{ fontFamily: "Consolas, 'Courier New', monospace" }}
        >
            <div className="flex gap-2">
                <div>{isFolded ? "(+)" : "(-)"}</div>
                <span
                    className="cursor-pointer"
                    onClick={() => setIsFolded((isFolded) => !isFolded)}
                >
                    <b>alloc_id:</b> {node.alloc_id ?? "none"}, <b>offset: </b>
                    {node.offset ?? "none"}, <b>ty: </b>{" "}
                    <TextFold text={node.ty ?? "none"} maxLength={50} />
                </span>
            </div>
            {node.info_messages.length > 0 && (
                <div className="text-blue-600 ml-8 pl-2 border-l border-blue-600">
                    {node.info_messages.map((message) => (
                        <p className="my-0">
                            <TextFold text={message} maxLength={80} />
                        </p>
                    ))}
                </div>
            )}
            {node.error_messages.length > 0 && (
                <div className="text-red-600 ml-8 pl-2 border-l border-red-600">
                    {node.error_messages.map((message) => (
                        <p className="my-0">
                            <TextFold text={message} maxLength={80} />
                        </p>
                    ))}
                </div>
            )}
            <div
                className="ml-8"
                style={{
                    display: isFolded ? "none" : "block",
                }}
            >
                {node.children.map((child) => (
                    <Visualizer node={child} />
                ))}
            </div>
        </div>
    );
};

const RemoteFileSelector: React.FC<{
    isShown: boolean;
    onFileSelected?: (fileContent: string) => void;
    onShouldClose?: () => void;
}> = ({ isShown, onFileSelected, onShouldClose }) => {
    const { data, isLoading } = useSWR<{ filename: string; size: number }[]>(
        "remote_files",
        (_: string) => {
            return fetch("/api/files").then((res) => res.json());
        }
    );

    const openFile = useCallback(async (filename: string) => {
        const url = new URL(document.location.href);
        url.pathname = "/api/file";
        url.searchParams.append("filename", filename);

        const json = await fetch(url).then((res) => res.text());
        onFileSelected?.(json);
        onShouldClose?.();
    }, []);

    const [[field, descending], setSortCriterion] = useState<
        [field: "filename" | "size", descending: boolean]
    >(["size", true]);

    const onClickHeader = (field: "filename" | "size") => {
        setSortCriterion(([originalField, originalDescending]) => [
            field,
            field === originalField ? !originalDescending : false,
        ]);
    };

    const sortedFiles = useMemo(() => {
        return data?.sort((file1, file2) => {
            return (
                (descending ? -1 : 1) * (file1[field] < file2[field] ? -1 : 1)
            );
        });
    }, [data, field, descending]);

    return !isShown ? (
        <></>
    ) : (
        <div className="fixed left-0 top-0 h-screen w-screen flex justify-center items-center">
            <div className="w-[600px] max-w-[calc(100dvw-16px)] max-h-[calc(100dvh-16px)] border border-black bg-white rounded-lg py-2 px-4 overflow-auto">
                <div className="grid grid-cols-2 gap-x-4">
                    <div
                        onClick={() => onClickHeader("filename")}
                        className="cursor-pointer sticky top-0 bg-white"
                    >
                        <b>
                            Filename{" "}
                            {field === "filename" && (!descending ? "▲ " : "▼")}
                        </b>
                    </div>
                    <div
                        onClick={() => onClickHeader("size")}
                        className="cursor-pointer sticky top-0 bg-white"
                    >
                        <b>
                            Size{" "}
                            {field === "size" && (!descending ? "▲ " : "▼")}
                        </b>
                    </div>
                    {sortedFiles?.map(({ filename, size }) => (
                        <>
                            <div
                                className="cursor-pointer"
                                onClick={() => openFile(filename)}
                            >
                                {filename}
                            </div>
                            <div>{size} bytes</div>
                        </>
                    ))}
                </div>
            </div>
        </div>
    );
};

export const App = () => {
    const fileRef = useRef<HTMLInputElement>(null);
    const [node, setNode] = useState<Node | undefined>(undefined);
    const [isRemoteFileSelectorShown, setIsRemoteFileSelectorShown] =
        useState(false);

    const onFileChange: React.ChangeEventHandler<HTMLInputElement> = async (
        event
    ) => {
        if (event.target.files && event.target.files.length > 0) {
            const file = event.target.files[0];
            setNode(JSON.parse(await readFileToString(file)));
        } else {
            setNode(undefined);
        }

        event.target.value = "";
    };

    const onRemoteFileSelected = async (content: string) => {
        setNode(JSON.parse(content));
    };

    return (
        <div className="py-2 px-4">
            <button
                className="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800"
                type="button"
                onClick={() => {
                    fileRef.current?.click();
                }}
            >
                Select local file
            </button>
            <button
                className="focus:outline-none text-white bg-purple-700 hover:bg-purple-800 focus:ring-4 focus:ring-purple-300 font-medium rounded-lg text-sm px-5 py-2.5 mb-2 dark:bg-purple-600 dark:hover:bg-purple-700 dark:focus:ring-purple-900"
                type="button"
                onClick={() => void setIsRemoteFileSelectorShown(true)}
            >
                Select remote file
            </button>
            <input
                className="hidden"
                type="file"
                ref={fileRef}
                onChange={onFileChange}
            />
            <div>
                {node ? <Visualizer node={node} /> : "Please choose a file."}
            </div>
            <RemoteFileSelector
                isShown={isRemoteFileSelectorShown}
                onFileSelected={onRemoteFileSelected}
                onShouldClose={() => void setIsRemoteFileSelectorShown(false)}
            />
        </div>
    );
};
