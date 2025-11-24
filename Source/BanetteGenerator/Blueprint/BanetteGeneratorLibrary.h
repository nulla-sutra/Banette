// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "BanetteGeneratorLibrary.generated.h"

/**
 * 
 */
UCLASS()
class BANETTEGENERATOR_API UBanetteGeneratorLibrary : public UBlueprintFunctionLibrary
{
	GENERATED_BODY()

public:
	UFUNCTION(BlueprintCallable)
	static void GenerateOpenApi(FString OpenApiPath, FString OutputDir, FString FileName, FString ModuleName);
};
