using UnrealBuildTool;

public class BanetteTransport : ModuleRules
{
    public BanetteTransport(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicIncludePaths.Add(ModuleDirectory);

        PublicDependencyModuleNames.AddRange(
            new string[]
            {
                "Core",
                "CoreUObject",
                "HTTP",
                "UE5Coro",
                "BanetteCore"
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