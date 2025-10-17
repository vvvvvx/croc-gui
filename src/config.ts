export interface AppConfig {
  transfers: number;
  zip:boolean;
  exclude:string;
  overwrite: boolean;
  multicast: string;
  ip: string;
  relay: string;
  relay6: string;
  pass: string;
}

export const defaultConfig: AppConfig = {
  transfers: 8,
  zip: false,
  exclude: "",
  overwrite: false,
  multicast: "239.255.255.250",
  ip: "",
  relay: "",
  relay6: "",
  pass: "",
};
