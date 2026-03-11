"use client";

import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';

interface AssetPreferencesStepProps {
  data: {
    asset_types: string[]; // Array of selected asset types
  };
  onChange: (data: Partial<AssetPreferencesStepProps['data']>) => void;
}

const ASSET_TYPES = [
  { value: 'Stocks', label: 'Stocks' },
  { value: 'Options', label: 'Options' },
  { value: 'Crypto', label: 'Cryptocurrency' },
  { value: 'Forex', label: 'Forex' },
  { value: 'Futures', label: 'Futures' },
];

export function AssetPreferencesStep({ data, onChange }: AssetPreferencesStepProps) {
  const toggleAssetType = (assetType: string) => {
    const current = data.asset_types || [];
    const updated = current.includes(assetType)
      ? current.filter((t) => t !== assetType)
      : [...current, assetType];
    onChange({ asset_types: updated });
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label>What do you trade? *</Label>
        <p className="text-sm text-muted-foreground">
          Select all asset types you trade or plan to trade
        </p>
      </div>

      <div className="space-y-3">
        {ASSET_TYPES.map((asset) => (
          <div key={asset.value} className="flex items-center space-x-2">
            <Checkbox
              id={asset.value}
              checked={(data.asset_types || []).includes(asset.value)}
              onCheckedChange={() => toggleAssetType(asset.value)}
            />
            <Label
              htmlFor={asset.value}
              className="font-normal cursor-pointer"
            >
              {asset.label}
            </Label>
          </div>
        ))}
      </div>

      {(!data.asset_types || data.asset_types.length === 0) && (
        <p className="text-sm text-destructive">
          Please select at least one asset type
        </p>
      )}
    </div>
  );
}

