import { invoke } from "@tauri-apps/api/core";

// setting input valid regex
export const reIPv4          = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/;
export const reIPv4Port      = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(:(6553[0-5]|655[0-2]\d|65[0-4]\d{2}|6[0-4]\d{3}|[1-5]?\d{1,4}))$/;
export const reIPv6MustPort  = /^\[(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(([0-9a-fA-F]{1,4}:){1,7}:)|(([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4})|(([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2})|(([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3})|(([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4})|(([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5})|([0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6}))|(:((:[0-9a-fA-F]{1,4}){1,7}|:)))(%.+)?\](?::([0-9]{1,5}))$/;
export const reIPv6NoPort    = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(([0-9a-fA-F]{1,4}:){1,7}:)|(([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4})|(([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2})|(([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3})|(([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4})|(([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5})|([0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6}))|(:((:[0-9a-fA-F]{1,4}){1,7}|:)))(%.+)?$/;
export const reIPv4Multicast = /^(22[4-9]|23\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/;
export const reExclude       = /^[A-Za-z0-9\u4e00-\u9fa5]+(\s*,\s*[A-Za-z0-9\u4e00-\u9fa5]+)*$/;
export const reNum           = /^[1-9][0-9]?$/;

export interface AppConfig  {
  transfers: number,   // 多少个port同时传输
  //zip: boolean,        // 发送前是否先打包压缩
  //exclude: string,     // 排除哪些文件，以逗号分隔
  overwrite: boolean,  // 是否自动覆盖或续传
  multicast: string,   // 局域网广播范围 默认：239.255.255.250
  ip: string,          // 本机IP,如果有
  local:boolean,       // 强制本地连接
  relay: string,       // IP v4中继 
  relay6: string,      // IP v6中继
  relay_passwd: string, // 中继密码
  proxy_socks5: string, 
  proxy_http: string,
}

export async function loadConfig():Promise<AppConfig>{

  const cfg= await invoke("load_config");
  return cfg as AppConfig;
}

export async function saveConfig(cfg:AppConfig){

  await invoke("save_config",{cfg:cfg});

}
export function isEmpty(str:string):boolean{
  return String(str).trim().length<=0;
}
export function isIPv4(strIP:string):boolean{
  return reIPv4.test(String(strIP).trim());
}
export function isIPv4Port(strIP:string):boolean{
  return reIPv4Port.test(String(strIP).trim());
}
export function isIPv6(strIP:string):boolean{
  return reIPv6NoPort.test(String(strIP).trim());
}
export function isIPv6Port(strIP:string):boolean{
  return reIPv6MustPort.test(String(strIP).trim());
}
export function isMulticast(strIP:string):boolean{
  return reIPv4Multicast.test(String(strIP).trim());
}
export function isProxy(strIP:string):boolean{
  return isIPv4Port(strIP) || isIPv6Port(strIP);
}
export function isIP(strIP:string):boolean{
  return isIPv4(strIP) || isIPv6(strIP);
}
export function isNum(str:string):boolean{
  return reNum.test(String(str).trim());
}
export function isExclude(str:string):boolean{
  return reExclude.test(String(str).trim());
}
