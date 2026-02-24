const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');
const axios = require('axios');

const PROVIDERS_PATH = path.join(__dirname, '../agent-diva-providers/src/providers.yaml');

// Configuration for providers to update
const UPDATERS = {
  openai: async () => {
    // Example: Fetch from OpenAI API
    // const response = await axios.get('https://api.openai.com/v1/models', {
    //   headers: { Authorization: `Bearer ${process.env.OPENAI_API_KEY}` }
    // });
    // return response.data.data.map(m => m.id).filter(id => id.startsWith('gpt-'));
    return null; // Not implemented
  },
  openrouter: async () => {
    try {
        const response = await axios.get('https://openrouter.ai/api/v1/models');
        return response.data.data.map(m => m.id);
    } catch (e) {
        console.error('Failed to fetch OpenRouter models', e.message);
        return null;
    }
  }
  // Add more providers here
};

async function main() {
  try {
    // 1. Read existing config
    const fileContent = fs.readFileSync(PROVIDERS_PATH, 'utf8');
    const config = yaml.load(fileContent);

    let hasChanges = false;

    // 2. Update each provider
    for (const provider of config) {
      if (UPDATERS[provider.name]) {
        console.log(`Updating ${provider.name}...`);
        const newModels = await UPDATERS[provider.name]();
        
        if (newModels && newModels.length > 0) {
          // Merge logic: Add new models, keep existing ones? Or replace?
          // Here we assume we want to append new ones that don't exist
          const existingModels = new Set(provider.models || []);
          let addedCount = 0;
          
          for (const model of newModels) {
            if (!existingModels.has(model)) {
              provider.models.push(model);
              existingModels.add(model);
              addedCount++;
            }
          }
          
          if (addedCount > 0) {
            console.log(`Added ${addedCount} models to ${provider.name}`);
            hasChanges = true;
          }
        }
      }
    }

    // 3. Save back if changed
    if (hasChanges) {
      const newYaml = yaml.dump(config, { lineWidth: -1 });
      fs.writeFileSync(PROVIDERS_PATH, newYaml, 'utf8');
      console.log('Successfully updated providers.yaml');
    } else {
      console.log('No changes detected.');
    }

  } catch (error) {
    console.error('Error updating models:', error);
    process.exit(1);
  }
}

main();
