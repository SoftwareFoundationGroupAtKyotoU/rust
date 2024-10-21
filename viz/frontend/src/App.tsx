import React, { useMemo, useRef, useState } from "react";
import {
    serializeKey,
    VisualizerContext,
    VisualizerData,
    VisualizerNodeKey,
    VisualizerNodeKeySerialized,
    VisualizerNodeValue,
} from "./types";
import { readFileToString } from "./utils";
import { Visualizer } from "./parts/Visualizer";
import { RemoteFileSelector } from "./parts/RemoteFileSelector";

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
