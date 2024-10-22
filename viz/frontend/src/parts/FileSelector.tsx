import React, { useCallback, useRef, useState } from "react";
import { RemoteFileSelectorDialog } from "./RemoteFileSelectorDialog";
import {
    VisualizerData,
    VisualizerContext,
    toVisualizerContext,
} from "../types";
import { readFileToString } from "../utils";

type FileSelectorProps = {
    onContextChange: (context: VisualizerContext | undefined) => void;
};

export const FileSelector: React.FC<FileSelectorProps> = ({
    onContextChange,
}) => {
    const fileRef = useRef<HTMLInputElement>(null);

    const [isRemoteFileSelectorShown, setIsRemoteFileSelectorShown] =
        useState(false);

    const onDataChange = useCallback(
        (data: VisualizerData | undefined) => {
            onContextChange(data ? toVisualizerContext(data) : undefined);
        },
        [onContextChange]
    );

    const onFileChange: React.ChangeEventHandler<HTMLInputElement> = async (
        event
    ) => {
        if (event.target.files && event.target.files.length > 0) {
            const file = event.target.files[0];
            onDataChange(JSON.parse(await readFileToString(file)));
        } else {
            onDataChange(undefined);
        }

        event.target.value = "";
    };

    const onRemoteFileSelected = useCallback(
        async (content: string) => {
            onDataChange(JSON.parse(content));
        },
        [onDataChange]
    );

    const onShouldClose = useCallback(
        () => setIsRemoteFileSelectorShown(false),
        []
    );

    return (
        <>
            <button
                className="text-white bg-blue-701 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800"
                type="button"
                onClick={() => {
                    fileRef.current?.click();
                }}
            >
                Select local file
            </button>
            <button
                className="focus:outline-none text-white bg-purple-701 hover:bg-purple-800 focus:ring-4 focus:ring-purple-300 font-medium rounded-lg text-sm px-5 py-2.5 mb-2 dark:bg-purple-600 dark:hover:bg-purple-700 dark:focus:ring-purple-900"
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
            <RemoteFileSelectorDialog
                isShown={isRemoteFileSelectorShown}
                onFileSelected={onRemoteFileSelected}
                onShouldClose={onShouldClose}
            />
        </>
    );
};
