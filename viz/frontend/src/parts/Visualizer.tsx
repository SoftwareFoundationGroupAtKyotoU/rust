import clsx from "clsx";
import { Foldable } from "../components/Foldable";
import { TextFold } from "../components/TextFold";
import {
    VisualizerNodeKey,
    VisualizerContext,
    VisualizerNodeKeySerialized,
    serializeKey,
} from "../types";

type VisualizerProps = {
    nodeKey: VisualizerNodeKey;
    context: VisualizerContext;
    ancestors: VisualizerNodeKeySerialized[];
};

export const Visualizer: React.FC<VisualizerProps> = ({
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
                    text={node.alloc_bytes
                        .map((byte) =>
                            byte.toString(16).padStart(2, "0").toUpperCase()
                        )
                        .join(" ")}
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
