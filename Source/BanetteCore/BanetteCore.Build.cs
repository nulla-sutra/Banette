using UnrealBuildTool;

public class BanetteCore : ModuleRules
{
    public BanetteCore(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicIncludePaths.Add(ModuleDirectory);

        PublicDependencyModuleNames.AddRange(
            new string[]
            {
                "Core",
                "CoreUObject",
                "UE5Coro"
            }
        );

        PrivateDependencyModuleNames.AddRange(
            new string[]
            {
                "Engine"
            }
        );
    }
}