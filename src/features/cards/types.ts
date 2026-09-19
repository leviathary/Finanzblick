// Definiert den stabilen Datenaustausch des Karten-Features mit dem Backend.
import type { AccountType } from "../../domain/finance";
export interface Account { id:number; name:string; accountType:AccountType; isActive:boolean; currency:string }
export interface Row { id:number; bookingDate:string; description:string; amountMinor:number; currency:string; kind:string; categoryKey:string; suggestedSettlement:boolean; manuallyOverridden:boolean }
export interface Category { key:string; label:string }
export interface SettlementRule { id:number; accountId:number; accountName:string; currency:string; direction:number; prefix:string }
export interface AccountRow extends Row { accountId:number; accountName:string }
export interface Decision { transactionId:number; kind:"REFUND"|"UNKNOWN"; categoryKey:string|null }
export interface Preview { prefix:string; accountName:string; currency:string; direction:number; protectedCount:number; matches:{id:number;bookingDate:string;description:string;amountMinor:number}[] }

export interface SetupRule { transactionId: number; prefix: string; expectedIds: number[] }
export interface SetupRequest { cardId: number; cardRule: SetupRule | null; bankRule: SetupRule | null; decisions: Decision[]; past: boolean; future: boolean }
