import React, { useCallback, useMemo, useRef, useState } from "react";
import clsx from "clsx";
import useSWR from "swr";

function readFileToString(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => resolve(event.target?.result as string);
        reader.onerror = (error) => reject(error);
        reader.readAsText(file);
    });
}

type VisualizerNodeKey = { alloc_id: number; offset: number; ty: string };

const serializeKey = (key: VisualizerNodeKey): VisualizerNodeKeySerialized =>
    JSON.stringify([
        key.alloc_id,
        key.offset,
        key.ty,
    ]) as VisualizerNodeKeySerialized;

type VisualizerNodeValue = {
    alloc_bytes: number[];
    messages: { severity: string; message: string }[];
};

type VisualizerFrame = { nodes: VisualizerNodeKey[] };

type VisualizerData = {
    nodes: [VisualizerNodeKey, VisualizerNodeValue][];
    edges: [VisualizerNodeKey, VisualizerNodeKey][];
    frames: VisualizerFrame[];
};

type VisualizerNodeKeySerialized = string & { __type: "KEY" };

/** VisualizerData but processed */
type VisualizerContext = {
    nodes: Record<VisualizerNodeKeySerialized, VisualizerNodeValue>;
    edges: Record<VisualizerNodeKeySerialized, VisualizerNodeKey[]>;
    frames: VisualizerFrame[];
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

type VisualizerProps = {
    nodeKey: VisualizerNodeKey;
    context: VisualizerContext;
    ancestors: VisualizerNodeKeySerialized[];
};

const Foldable: React.FC<
    React.PropsWithChildren<{ header: React.ReactNode }>
> = ({ header, children }) => {
    const [isFolded, setIsFolded] = useState(false);

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
                className="ml-8"
                style={{
                    display: isFolded ? "none" : "block",
                }}
            >
                {children}
            </div>
        </div>
    );
};

const Visualizer: React.FC<VisualizerProps> = ({
    nodeKey,
    context,
    ancestors,
}) => {
    const nodeKeySerialized = serializeKey(nodeKey);
    const node = context.nodes[nodeKeySerialized];

    const header = (
        <>
            {" "}
            <b>alloc_id:</b> {nodeKey.alloc_id ?? "none"}, <b>offset: </b>
            {nodeKey.offset ?? "none"}, <b>ty: </b>{" "}
            <TextFold text={nodeKey.ty ?? "none"} maxLength={50} />
        </>
    );

    if (ancestors.includes(nodeKeySerialized)) {
        return (
            <a
                className="text-green-600 border-green-600"
                href={`#node_${nodeKeySerialized}`}
            >
                (loop) {header}
            </a>
        );
    }

    return (
        <Foldable header={header}>
            <a id={`node_${nodeKeySerialized}`}></a>
            <div>
                bytes:{" "}
                <TextFold
                    text={JSON.stringify(node.alloc_bytes)}
                    maxLength={80}
                />
            </div>
            {node.messages.length > 0 && (
                <>
                    {node.messages.map((message) => (
                        <div
                            className={clsx(
                                "ml-8 pl-2 border-l text-blue-600 border-blue-600",
                                {
                                    "text-blue-600 border-blue-600":
                                        message.severity === "INFO",
                                    "text-red-600 border-red-600":
                                        message.severity === "ERROR",
                                }
                            )}
                        >
                            <p className="my-0">
                                <TextFold
                                    text={message.message}
                                    maxLength={80}
                                />
                            </p>
                        </div>
                    ))}
                </>
            )}
            {context.edges[nodeKeySerialized]?.map((child) => (
                <Visualizer
                    nodeKey={child}
                    context={context}
                    ancestors={[...ancestors, nodeKeySerialized]}
                />
            ))}
        </Foldable>
    );
};

const RemoteFileSelector: React.FC<{
    isShown: boolean;
    onFileSelected?: (fileContent: string) => void;
    onShouldClose?: () => void;
}> = ({ isShown, onFileSelected, onShouldClose }) => {
    const [searchQuery, setSearchQuery] = useState("");

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

    const filteredFiles = useMemo(() => {
        return sortedFiles?.filter((file) =>
            file.filename.includes(searchQuery)
        );
    }, [sortedFiles, searchQuery]);

    return !isShown ? (
        <></>
    ) : (
        <div className="fixed left-0 top-0 h-screen w-screen flex justify-center items-center">
            <div className="w-[600px] max-w-[calc(100dvw-16px)] max-h-[calc(100dvh-16px)] border border-black bg-white rounded-lg py-2 px-4 overflow-auto">
                <div className="grid grid-cols-2 gap-x-4">
                    <div className="col-span-2">
                        <input
                            type="text"
                            className="border border-black w-full"
                            value={searchQuery}
                            onChange={(event) =>
                                setSearchQuery(event.target.value)
                            }
                        />
                    </div>
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
                    {filteredFiles?.slice(0, 500).map(({ filename, size }) => (
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
                    {filteredFiles !== undefined &&
                        filteredFiles.length > 500 && (
                            <div className="col-span-2 text-center text-gray-500">
                                Only first 500 results shown.
                            </div>
                        )}
                </div>
            </div>
        </div>
    );
};

export const App = () => {
    const fileRef = useRef<HTMLInputElement>(null);
    const [data, setData] = useState<VisualizerData | undefined>(undefined);
    const context: VisualizerContext | undefined = useMemo(() => {
        if (!data) return;
        return {
            nodes: Object.fromEntries(
                data.nodes.map(([key, value]) => [serializeKey(key), value])
            ) as Record<VisualizerNodeKeySerialized, VisualizerNodeValue>,
            edges: data.edges
                .map(
                    ([key, value]) =>
                        [serializeKey(key), value] as [
                            VisualizerNodeKeySerialized,
                            VisualizerNodeKey
                        ]
                )
                .reduce(
                    (
                        map: Record<
                            VisualizerNodeKeySerialized,
                            VisualizerNodeKey[]
                        >,
                        [key, value]
                    ) => {
                        map[key] ??= [];
                        map[key].push(value);
                        return map;
                    },
                    {}
                ) as Record<VisualizerNodeKeySerialized, VisualizerNodeKey[]>,
            frames: data.frames,
        };
    }, [data]);
    const [isRemoteFileSelectorShown, setIsRemoteFileSelectorShown] =
        useState(false);

    const onFileChange: React.ChangeEventHandler<HTMLInputElement> = async (
        event
    ) => {
        if (event.target.files && event.target.files.length > 0) {
            const file = event.target.files[0];
            setData(JSON.parse(await readFileToString(file)));
        } else {
            setData(undefined);
        }

        event.target.value = "";
    };

    const onRemoteFileSelected = async (content: string) => {
        setData(JSON.parse(content));
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
                {context?.frames.map((frame) => (
                    <div>
                        {frame.nodes.map((node) => (
                            <Visualizer
                                nodeKey={node}
                                context={context}
                                ancestors={[]}
                            />
                        ))}
                    </div>
                ))}
            </div>
            <RemoteFileSelector
                isShown={isRemoteFileSelectorShown}
                onFileSelected={onRemoteFileSelected}
                onShouldClose={() => void setIsRemoteFileSelectorShown(false)}
            />
        </div>
    );
};
