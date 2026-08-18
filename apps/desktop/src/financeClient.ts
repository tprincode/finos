import { invoke } from "@tauri-apps/api/core";
import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type CommandRequest,
  type CommandResult,
  type QueryRequest,
  type QueryResult,
} from "@finos/app-contracts";

export class LocalTauriFinanceClient {
  contractVersion(): string {
    return FINANCE_CLIENT_CONTRACT_VERSION;
  }

  executeQuery(queryName: string, body?: unknown): Promise<QueryResult> {
    const request: QueryRequest = {
      contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
      queryName,
      correlationId: crypto.randomUUID(),
      bodyJson: body === undefined ? undefined : JSON.stringify(body),
    };
    return invoke<QueryResult>("finance_query", { request });
  }

  executeCommand(commandName: string, body?: unknown): Promise<CommandResult> {
    const request: CommandRequest = {
      contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
      commandName,
      correlationId: crypto.randomUUID(),
      bodyJson: body === undefined ? undefined : JSON.stringify(body),
    };
    return invoke<CommandResult>("finance_command", { request });
  }
}
