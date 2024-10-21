import React, { useMemo, useRef, useState } from "react";
import {
    toVisualizerContext,
    VisualizerAllocId,
    VisualizerContext,
    VisualizerData,
} from "./types";
import { readFileToString } from "./utils";
import { Visualizer } from "./parts/Visualizer";
import { RemoteFileSelector } from "./parts/RemoteFileSelector";
import { Foldable } from "./components/Foldable";
import clsx from "clsx";

export const App = () => {
    const fileRef = useRef<HTMLInputElement>(null);
    const [data, setData] = useState<VisualizerData | undefined>(undefined);

    const context: VisualizerContext | undefined = useMemo(
        () => (data ? toVisualizerContext(data) : undefined),
        [data]
    );

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
            <Foldable header="Alloc list">
                <div>
                    {Object.entries(context?.allocs ?? {}).map(
                        ([allocId, alloc]) => (
                            <div
                                className={clsx({
                                    "text-red-600":
                                        !context?.reachableAllocIds.has(
                                            +allocId as VisualizerAllocId
                                        ),
                                    "text-green-600":
                                        context?.reachableAllocIds.has(
                                            +allocId as VisualizerAllocId
                                        ),
                                })}
                            >
                                <Foldable
                                    header={`Alloc ${allocId} (${alloc.bytes.length} bytes)`}
                                >
                                    {/* Check reachable or not through the alloc graph */}
                                    <div className="flex gap-[1ch]">
                                        <div>bytes:</div>
                                        <div>
                                            {alloc.bytes
                                                .map((byte) =>
                                                    byte
                                                        .toString(16)
                                                        .padStart(2, "0")
                                                        .toUpperCase()
                                                )
                                                .join(" ")}
                                        </div>
                                    </div>
                                </Foldable>
                            </div>
                        )
                    )}
                </div>
            </Foldable>
            <Foldable header={<>Alloc graph</>}>
                <div>
                    {context?.frames.map((frame) => (
                        <Foldable header={<>frame {frame.description}</>}>
                            {frame.nodes.map((node) => (
                                <Visualizer
                                    nodeKey={node}
                                    context={context}
                                    ancestors={[]}
                                />
                            ))}
                        </Foldable>
                    ))}
                </div>
                <RemoteFileSelector
                    isShown={isRemoteFileSelectorShown}
                    onFileSelected={onRemoteFileSelected}
                    onShouldClose={() =>
                        void setIsRemoteFileSelectorShown(false)
                    }
                />
            </Foldable>
        </div>
    );
};
