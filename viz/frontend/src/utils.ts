export function readFileToString(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => resolve(event.target?.result as string);
        reader.onerror = (error) => reject(error);
        reader.readAsText(file);
    });
}

export function chunk<T>(array: T[], size: number): T[][] {
    if (size <= 0) {
        throw new Error("Chunk size must be greater than 0");
    }

    const chunkedArray: T[][] = [];
    for (let i = 0; i < array.length; i += size) {
        chunkedArray.push(array.slice(i, i + size));
    }

    return chunkedArray;
}
