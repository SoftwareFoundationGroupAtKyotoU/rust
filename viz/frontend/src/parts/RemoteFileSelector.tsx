import { useState, useCallback, useMemo } from "react";
import useSWR from "swr";

export const RemoteFileSelector: React.FC<{
    isShown: boolean;
    onFileSelected?: (fileContent: string) => void;
    onShouldClose?: () => void;
}> = ({ isShown, onFileSelected, onShouldClose }) => {
    const [searchQuery, setSearchQuery] = useState("");

    const { data, isLoading: _ } = useSWR<{ filename: string; size: number }[]>(
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
