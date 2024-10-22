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

export function tally<T extends string | number | symbol>(
    items: T[]
): Record<T, number> {
    return items.reduce((acc, item) => {
        acc[item] = (acc[item] ?? 0) + 1;
        return acc;
    }, {} as Record<T, number>);
}
