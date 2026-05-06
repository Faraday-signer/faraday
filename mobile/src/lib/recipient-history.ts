import AsyncStorage from "@react-native-async-storage/async-storage";

const STORAGE_KEY = "faraday:recipients:v1";
const MAX_HISTORY = 50;

async function storageGet(): Promise<string[]> {
  const raw = await AsyncStorage.getItem(STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((s): s is string => typeof s === "string");
  } catch {
    return [];
  }
}

async function storageSet(list: string[]): Promise<void> {
  await AsyncStorage.setItem(STORAGE_KEY, JSON.stringify(list));
}

export async function getRecipientHistory(): Promise<string[]> {
  return storageGet();
}

export async function recordRecipient(address: string): Promise<void> {
  const trimmed = address.trim();
  if (trimmed.length === 0) return;
  const current = await storageGet();
  const next = [trimmed, ...current.filter((a) => a !== trimmed)].slice(0, MAX_HISTORY);
  await storageSet(next);
}

export async function clearRecipientHistory(): Promise<void> {
  await storageSet([]);
}
