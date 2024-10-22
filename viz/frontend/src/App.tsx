import { useState } from "react";
import { VisualizerContext } from "./types";
import { Visualizer } from "./parts/Visualizer";
import { FileSelector } from "./parts/FileSelector";

export const App = () => {
    const [context, setContext] = useState<VisualizerContext | undefined>(
        undefined
    );

    return (
        <div className="py-2 px-4">
            <FileSelector
                onContextChange={(context) => {
                    setContext(context);
                }}
            />

            <div>
                {context ? (
                    <Visualizer context={context} />
                ) : (
                    "Please select a file."
                )}
            </div>
        </div>
    );
};
