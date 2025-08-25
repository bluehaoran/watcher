import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { logger } from '../utils/logger';

export interface BaseEntity {
  id: string;
  createdAt: string;
  updatedAt: string;
}

export interface QueryOptions {
  limit?: number;
  offset?: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

export class FileStore<T extends BaseEntity> {
  private dataPath: string;
  private cache = new Map<string, T>();
  private dirty = false;
  private saveTimeout: NodeJS.Timeout | null = null;

  constructor(private tableName: string, private dataDir = './data') {
    this.dataPath = join(dataDir, `${tableName}.json`);
  }

  async init(): Promise<void> {
    try {
      if (!existsSync(this.dataDir)) {
        await mkdir(this.dataDir, { recursive: true });
        logger.info(`Created data directory: ${this.dataDir}`);
      }
      await this.load();
      logger.info(`Initialized ${this.tableName} store with ${this.cache.size} records`);
    } catch (error) {
      logger.error(`Failed to initialize ${this.tableName} store:`, error);
      throw error;
    }
  }

  private async load(): Promise<void> {
    try {
      if (existsSync(this.dataPath)) {
        const data = await readFile(this.dataPath, 'utf-8');
        const records: T[] = JSON.parse(data);
        this.cache.clear();
        records.forEach(record => {
          // Ensure required fields exist
          if (!record.id) record.id = randomUUID();
          if (!record.createdAt) record.createdAt = new Date().toISOString();
          if (!record.updatedAt) record.updatedAt = record.createdAt;
          this.cache.set(record.id, record);
        });
        logger.debug(`Loaded ${records.length} records from ${this.tableName}`);
      }
    } catch (error) {
      logger.warn(`Failed to load ${this.tableName}:`, error);
    }
  }

  private async save(): Promise<void> {
    if (!this.dirty) return;
    
    try {
      const data = Array.from(this.cache.values());
      await writeFile(this.dataPath, JSON.stringify(data, null, 2));
      this.dirty = false;
      logger.debug(`Saved ${data.length} records to ${this.tableName}`);
    } catch (error) {
      logger.error(`Failed to save ${this.tableName}:`, error);
      throw error;
    }
  }

  private scheduleSave(): void {
    if (this.saveTimeout) return;
    
    this.saveTimeout = setTimeout(async () => {
      try {
        await this.save();
      } finally {
        this.saveTimeout = null;
      }
    }, 100); // Debounce saves by 100ms
  }

  async create(data: Omit<T, 'id' | 'createdAt' | 'updatedAt'>): Promise<T> {
    const now = new Date().toISOString();
    const record = {
      ...data,
      id: randomUUID(),
      createdAt: now,
      updatedAt: now,
    } as T;

    this.cache.set(record.id, record);
    this.dirty = true;
    this.scheduleSave();
    
    logger.debug(`Created ${this.tableName} record: ${record.id}`);
    return record;
  }

  async findById(id: string): Promise<T | null> {
    return this.cache.get(id) || null;
  }

  async findMany(filter?: (record: T) => boolean, options: QueryOptions = {}): Promise<T[]> {
    let records = Array.from(this.cache.values());
    
    // Apply filter
    if (filter) {
      records = records.filter(filter);
    }
    
    // Apply sorting
    if (options.sortBy) {
      records.sort((a, b) => {
        const aVal = (a as any)[options.sortBy!];
        const bVal = (b as any)[options.sortBy!];
        
        let comparison = 0;
        if (aVal < bVal) comparison = -1;
        else if (aVal > bVal) comparison = 1;
        
        return options.sortOrder === 'desc' ? -comparison : comparison;
      });
    }
    
    // Apply pagination
    if (options.offset) {
      records = records.slice(options.offset);
    }
    if (options.limit) {
      records = records.slice(0, options.limit);
    }
    
    return records;
  }

  async findFirst(filter?: (record: T) => boolean): Promise<T | null> {
    const records = await this.findMany(filter, { limit: 1 });
    return records[0] || null;
  }

  async count(filter?: (record: T) => boolean): Promise<number> {
    if (!filter) return this.cache.size;
    return Array.from(this.cache.values()).filter(filter).length;
  }

  async update(id: string, data: Partial<Omit<T, 'id' | 'createdAt'>>): Promise<T | null> {
    const existing = this.cache.get(id);
    if (!existing) {
      logger.warn(`Attempted to update non-existent ${this.tableName} record: ${id}`);
      return null;
    }

    const updated = {
      ...existing,
      ...data,
      updatedAt: new Date().toISOString(),
    } as T;

    this.cache.set(id, updated);
    this.dirty = true;
    this.scheduleSave();
    
    logger.debug(`Updated ${this.tableName} record: ${id}`);
    return updated;
  }

  async upsert(id: string, createData: Omit<T, 'id' | 'createdAt' | 'updatedAt'>, updateData: Partial<Omit<T, 'id' | 'createdAt'>>): Promise<T> {
    const existing = await this.findById(id);
    if (existing) {
      return await this.update(id, updateData) as T;
    } else {
      const now = new Date().toISOString();
      const record = {
        ...createData,
        id,
        createdAt: now,
        updatedAt: now,
      } as T;
      
      this.cache.set(id, record);
      this.dirty = true;
      this.scheduleSave();
      
      logger.debug(`Upserted ${this.tableName} record: ${id}`);
      return record;
    }
  }

  async delete(id: string): Promise<boolean> {
    const deleted = this.cache.delete(id);
    if (deleted) {
      this.dirty = true;
      this.scheduleSave();
      logger.debug(`Deleted ${this.tableName} record: ${id}`);
    }
    return deleted;
  }

  async deleteMany(filter: (record: T) => boolean): Promise<number> {
    const records = Array.from(this.cache.values());
    const toDelete = records.filter(filter);
    
    toDelete.forEach(record => this.cache.delete(record.id));
    
    if (toDelete.length > 0) {
      this.dirty = true;
      this.scheduleSave();
      logger.debug(`Deleted ${toDelete.length} ${this.tableName} records`);
    }
    
    return toDelete.length;
  }

  async transaction<R>(fn: (store: FileStore<T>) => Promise<R>): Promise<R> {
    // Simple transaction implementation - save state and restore on error
    const originalCache = new Map(this.cache);
    const originalDirty = this.dirty;
    
    try {
      const result = await fn(this);
      await this.save(); // Force save transaction
      return result;
    } catch (error) {
      // Rollback
      this.cache = originalCache;
      this.dirty = originalDirty;
      logger.error(`Transaction failed for ${this.tableName}, rolled back:`, error);
      throw error;
    }
  }

  async flush(): Promise<void> {
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
      this.saveTimeout = null;
    }
    await this.save();
  }

  async reload(): Promise<void> {
    await this.load();
  }

  // Utility methods for common patterns
  exists(id: string): boolean {
    return this.cache.has(id);
  }

  all(): T[] {
    return Array.from(this.cache.values());
  }

  clear(): void {
    this.cache.clear();
    this.dirty = true;
    this.scheduleSave();
  }
}