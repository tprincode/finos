import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type CommandRequest,
  type CommandResult,
  type QueryRequest,
  type QueryResult,
} from "@finos/app-contracts";

function envelopeQuery(queryName: string, body?: unknown): QueryRequest {
  return {
    contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
    queryName,
    correlationId: crypto.randomUUID(),
    bodyJson: body === undefined ? undefined : JSON.stringify(body),
  };
}

function envelopeCommand(
  commandName: string,
  body?: unknown,
  expectedVersion?: number,
): CommandRequest {
  return {
    contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
    commandName,
    correlationId: crypto.randomUUID(),
    bodyJson: body === undefined ? undefined : JSON.stringify(body),
    expectedVersion,
  };
}

/** HTTP FinanceClient (ADR-0006). Same envelopes as desktop; Bearer JWT required by Axum. */
export class RemoteHttpFinanceClient {
  constructor(
    private readonly baseUrl: string,
    private readonly accessToken: string,
  ) {}

  contractVersion(): string {
    return FINANCE_CLIENT_CONTRACT_VERSION;
  }

  private headers(): HeadersInit {
    return {
      "content-type": "application/json",
      authorization: `Bearer ${this.accessToken}`,
    };
  }

  async executeQuery(queryName: string, body?: unknown): Promise<QueryResult> {
    const response = await fetch(`${this.baseUrl}/v1/queries`, {
      method: "POST",
      headers: this.headers(),
      body: JSON.stringify(envelopeQuery(queryName, body)),
    });
    if (!response.ok) {
      throw new Error(`RemoteHttpFinanceClient query failed: ${response.status}`);
    }
    return (await response.json()) as QueryResult;
  }

  async executeCommand(
    commandName: string,
    body?: unknown,
    expectedVersion?: number,
  ): Promise<CommandResult> {
    const response = await fetch(`${this.baseUrl}/v1/commands`, {
      method: "POST",
      headers: this.headers(),
      body: JSON.stringify(envelopeCommand(commandName, body, expectedVersion)),
    });
    if (!response.ok) {
      throw new Error(`RemoteHttpFinanceClient command failed: ${response.status}`);
    }
    return (await response.json()) as CommandResult;
  }
}
