export type ProxyEvent = {
  id: number;
  timestamp: number;
  method: string;
  uri: string;
  headers: [string, string][];
  is_response: boolean;
  status: number | null;
  body_base64: string | null;
};
