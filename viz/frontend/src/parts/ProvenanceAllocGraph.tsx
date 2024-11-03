import React from "react";
import { Foldable } from "../components/Foldable";
import {
    VisualizerAllocId,
    VisualizerContext,
    VisualizerProvenanceFrame,
} from "../types";

export type ProvenanceAllocGraphProps = {
    context: VisualizerContext;
    frame: VisualizerProvenanceFrame;
};

export const ProvenanceAllocGraph: React.FC<ProvenanceAllocGraphProps> = ({
    frame,
    context,
}) => (
    <Foldable header={<>{frame.description}</>}>
        {frame.nodes.map((allocId) => (
            <ProvenanceAllocNode
                context={context}
                allocId={allocId}
                ancestors={[]}
            />
        ))}
    </Foldable>
);

type ProvenanceAllocNodeProps = {
    context: VisualizerContext;
    allocId: VisualizerAllocId;
    ancestors: VisualizerAllocId[];
};

const ProvenanceAllocNode: React.FC<ProvenanceAllocNodeProps> = ({
    allocId,
    context,
    ancestors,
}) => (
    <Foldable header={<>Alloc {allocId}</>}>
        {!ancestors.includes(allocId) ? (
            context.provenanceGraph[allocId]?.map((childAllocId) => (
                <ProvenanceAllocNode
                    allocId={childAllocId}
                    context={context}
                    ancestors={[...ancestors, allocId]}
                />
            ))
        ) : (
            <>(Loop)</>
        )}
    </Foldable>
);
