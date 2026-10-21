export type AiAssistantPreferences = {
  enabled: boolean;
  provider: string;
  model: string;
  endpoint: string;
  candidate_limit: number;
  token?: string;
  tokens?: Record<string, string>;
  prompt_id?: string;
  prompt_custom_1: string;
  prompt_custom_2: string;
  prompt_custom_3: string;
  /** 英文模式（专用英文、临时英文）也把已输入的字母串发给 AI 服务做补全与拼写纠错（#9015）；默认关闭。 */
  english_candidates?: boolean;
};
