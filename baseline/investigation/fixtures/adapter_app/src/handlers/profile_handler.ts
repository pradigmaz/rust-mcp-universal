import { resolve_profile } from "../services/profile_service";

export async function resolveProfileHandler(profileId: string) {
  return resolve_profile(profileId);
}
