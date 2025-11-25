using UnrealBuildTool;

public class BanetteTransport : ModuleRules
{
	public BanetteTransport(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicIncludePaths.Add(ModuleDirectory);

		PublicDependencyModuleNames.AddRange(
			new[]
			{
				"Core",
				"CoreUObject",
				"HTTP",
				"UE5Coro",
				"Banette",
				"Json",
				"JsonUtilities"
			}
		);

		PrivateDependencyModuleNames.AddRange(
			new[]
			{
				"Engine"
			}
		);
	}
}