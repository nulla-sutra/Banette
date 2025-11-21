// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "JsonObjectWrapper.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "UE5Coro.h"
#include "BanetteTestLibrary.generated.h"

/**
 * 
 */
UCLASS()
class BANETTETEST_API UBanetteTestLibrary : public UBlueprintFunctionLibrary
{
	GENERATED_BODY()

	UFUNCTION(BlueprintCallable, Category = "Banette|Test", meta=(Latent, LatentInfo = LatentInfo))
	static FVoidCoroutine Test(FJsonObjectWrapper& Json, FLatentActionInfo LatentInfo);
};
